use esp32_nimble::{BLEDevice, BLEScan};
use esp_idf_hal::{
    adc::{
        attenuation::DB_11,
        oneshot::{
            config::AdcChannelConfig,
            AdcChannelDriver
        }
    }, 
    ledc::{
        config::TimerConfig,
        LedcDriver,
        LedcTimerDriver,
        Resolution
    },
    peripherals::Peripherals,
    task::block_on,
    prelude::*
};
use esp_idf_svc::log::set_target_level;
use parser::Value;
use std::{collections::HashSet, sync::{Arc, Mutex}, thread::{self, spawn}, time::Duration};
use ap::status;
use esp_idf_hal::adc::oneshot::AdcDriver;

mod ap;
mod miio;
mod net;
mod nvs;
mod parser;
mod serial;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    set_target_level("NimBLE", log::LevelFilter::Warn)?;
    set_target_level("BLE_INIT", log::LevelFilter::Warn)?;
    set_target_level("esp32_nimble", log::LevelFilter::Warn)?;

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let modem = peripherals.modem;

    let timer_driver = LedcTimerDriver::new(
        peripherals.ledc.timer1, 
        &TimerConfig::default()
            .frequency(50.Hz().into())
            .resolution(Resolution::Bits13)
    )?;

    let mut driver = LedcDriver::new(peripherals.ledc.channel0, timer_driver, pins.gpio9)?;
    let max_duty = driver.get_max_duty();

    driver.set_duty((max_duty as f32 * 0.025) as u32)?;

    let ble_device_cnt = Arc::new(Mutex::new(None::<usize>));
    let ble_device_cnt_clone = Arc::clone(&ble_device_cnt);

    let wifi_sta_cnt = Arc::new(Mutex::new(None::<u32>));
    let wifi_sta_cnt_clone = Arc::clone(&wifi_sta_cnt);

    let illumination = Arc::new(Mutex::new(None::<u16>));
    let illumination_clone = Arc::clone(&illumination);

    spawn(move || {
        let adc = AdcDriver::new(peripherals.adc1).unwrap();
        let mut adc_pin = AdcChannelDriver::new(&adc, pins.gpio1, &AdcChannelConfig {
            attenuation: DB_11,
            calibration: true,
            ..Default::default()
        }).unwrap();
        loop {
            thread::sleep(Duration::from_millis(500));
            match adc.read(&mut adc_pin) {
                Ok(value) => {
                    match *illumination.lock().unwrap() {
                        Some(last_value) => {
                            if last_value.abs_diff(value) <= 50 {
                                continue;
                            }
                        },
                        None => {}
                    }
                    *illumination.lock().unwrap() = Some(value);
                    log::info!("illumination: {}", value);
                },
                _ => continue
            }
        }
    });

    thread::Builder::new().stack_size(16 * 1024).spawn(move || {
        let mut net_manager = net::NetManager::new(modem).unwrap();

        loop {
            match net_manager.connect() {
                Ok(_) => {
                    log::info!("Connected to the network");
                    loop {
                        match status() {
                            Ok(data) => {
                                log::info!("AP status: {:?}", data);
                                *wifi_sta_cnt.lock().unwrap() = Some(data.ap.sta_count.parse::<u32>().unwrap());
                            }
                            Err(e) => {
                                log::error!("Failed to get AP status: {:?}", e);
                            }
                        }

                        std::thread::sleep(Duration::from_secs(10));

                        block_on(async {
                            log::info!("Start scanning BLE devices");
                            let ble_device = BLEDevice::take();
                            let mut ble_scan = BLEScan::new();
                            let mut ble_devices = HashSet::new();
                            ble_scan
                                .start(ble_device, 10000, |device, _| {
                                    ble_devices.insert(format!("{:?}", device.addr()));
                                    None::<()>
                                })
                                .await
                                .unwrap();
                            let cnt = ble_devices.len();
                            log::info!("Scanned BLE devices: {:?}", cnt);
                            *ble_device_cnt.lock().unwrap() = Some(cnt);
                        });

                        std::thread::sleep(Duration::from_secs(10));
                    }
                }
                Err(e) => {
                    log::error!("Failed to connect to the network: {:?}", e);
                    std::thread::sleep(Duration::from_secs(10));
                    continue;
                }
            }
        }
    })?;

    let mut miio = crate::miio::IoTFramework::new(
        peripherals.uart1, 
        pins.gpio12, 
        pins.gpio11,
        "csbupt.switch.smsw",
        "0001",
        "24351"
    )?;

    #[cfg(feature = "restore")]
    miio.restore()?;

    miio.registers(vec![
            (1, 1, "YouXam"),
            (1, 2, "csbupt.switch.smsw"),
            (1, 3, "0001"),
            (1, 4, "0001"),
        ])
        .registers(vec![
            (2, 2, 0), // 模式
            (2, 3, 0), // 故障
            (4, 1, 0), // 功耗参数
            (4, 2, 0), // 电功率
            (6, 1, 0), // Wifi 设备数量
            (7, 1, 0), // 蓝牙设备数量
            (8, 1, 0), // 亮度
        ])
        .registers(vec![
            (2, 4, false), // 防闪烁模式
            (4, 3, false), // 耗电量使用累加形式
            (5, 1, false), // 指示灯开关
        ])
        .register(2, 1, false)  // 开关
        .on(move |e| if let &Value::Boolean(value) = e {
            if value {
                log::info!("Set motor to 0.025");
                driver.set_duty((max_duty as f32 * 0.025) as u32).unwrap();
            } else {
                log::info!("Set motor to 0.043");
                driver.set_duty((max_duty as f32 * 0.043) as u32).unwrap();
            }
        })
        .register(7, 4, "") // 蓝牙设备名称
        .on(|value| {
            println!("bluetooth-devices: {}", value)
        })
        .load()?;

    miio.set_property(2, 1, Value::Boolean(true))?;

    #[allow(unused_must_use)]
    loop {
        miio.tick();

        if let Some(ble_device_cnt_) = *ble_device_cnt_clone.lock().unwrap() {
            miio.set_property(7, 1, Value::Integer(ble_device_cnt_ as u32));
        }
        if let Some(wifi_sta_cnt_) = *wifi_sta_cnt_clone.lock().unwrap() {
            miio.set_property(6, 1, Value::Integer(wifi_sta_cnt_ as u32));
        }
        if let Some(illumination) = *illumination_clone.lock().unwrap() {
            miio.set_property(8, 1, Value::Integer(illumination as u32));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

