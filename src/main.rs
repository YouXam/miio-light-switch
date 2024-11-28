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

    let ble_device_cnt = Arc::new(Mutex::new(None));
    let ble_device_cnt_clone = Arc::clone(&ble_device_cnt);

    let wifi_sta_cnt = Arc::new(Mutex::new(None));
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

    thread::Builder::new().stack_size(8 * 1024).spawn(move || {
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

    let mut serial = crate::serial::Serial::new(peripherals.uart1, pins.gpio12, pins.gpio11);

    #[cfg(feature = "restore")]
    serial.restore()?;

    serial.model("csbupt.switch.smsw")?;
    let _ = serial.version("0001", "24351");

    let mut miio = crate::miio::IoTFramework::new();

    // 企业标志
    miio.register_property(1, 1, Value::String("YouXam".to_string()), |_| {});
    // 产品模型
    miio.register_property(
        1,
        2,
        Value::String("csbupt.switch.smsw".to_string()),
        |_| {},
    );
    // 设备ID
    miio.register_property(1, 3, Value::String("0001".to_string()), |_| {});
    // 固件版本号
    miio.register_property(1, 4, Value::String("0001".to_string()), |_| {});

    // 开关
    miio.register_property(2, 1, Value::Boolean(false), move |e| match e {
        Value::Boolean(value) => {
            if *value {
                log::info!("Set motor to 0.025");
                driver.set_duty((max_duty as f32 * 0.025) as u32).unwrap();
            } else {
                log::info!("Set motor to 0.043");
                driver.set_duty((max_duty as f32 * 0.043) as u32).unwrap();
            }
        }
        _ => {}
    });
    // 模式
    miio.register_property(2, 2, Value::Integer(0), |_| {});
    // 故障
    miio.register_property(2, 3, Value::Integer(0), |_| {});
    // 防闪烁模式
    miio.register_property(2, 4, Value::Boolean(false), |_| {});

    // 功耗参数
    miio.register_property(4, 1, Value::Integer(0), |_| {});
    // 电功率
    miio.register_property(4, 2, Value::Integer(0), |_| {});
    // 耗电量使用累加形式
    miio.register_property(4, 3, Value::Boolean(false), |_| {});

    // 指示灯开关
    miio.register_property(5, 1, Value::Boolean(false), |_| {});

    // Wifi 设备数量
    miio.register_property(6, 1, Value::Integer(0), |_| {});

    // 蓝牙设备数量
    miio.register_property(7, 1, Value::Integer(0), |_| {});

    // 亮度
    miio.register_property(8, 1, Value::Integer(0), |_| {});

    match miio.set_property(2, 1, Value::Boolean(true)) {
        Some(result) => {
            let _ = serial.send(&result);
        },
        None => {}
    }

    loop {
        if let Ok(Some(event)) = serial.get_down() {
            match event {
                crate::serial::Event::SetProperties(props) => {
                    let response = miio.on_set_properties(props);
                    for r in response {
                        let _ = serial.send(&r);
                    }
                }
                crate::serial::Event::GetProperties(props) => {
                    let response = miio.on_get_properties(props);
                    let _ = serial.send(&response);
                }
                crate::serial::Event::Unknown => {}
            }
        }
        if let Some(ble_device_cnt_) = *ble_device_cnt_clone.lock().unwrap() {
            match miio.set_property(7, 1, Value::Integer(ble_device_cnt_ as u32)) {
                Some(result) => {
                    let _ = serial.send(&result);
                },
                None => {}
            }
        }
        if let Some(wifi_sta_cnt_) = *wifi_sta_cnt_clone.lock().unwrap() {
            match miio.set_property(6, 1, Value::Integer(wifi_sta_cnt_ as u32)) {
                Some(result) => {
                    let _ = serial.send(&result);
                },
                None => {}
            }
        }
        if let Some(illumination) = *illumination_clone.lock().unwrap() {
            match miio.set_property(8, 1, Value::Integer(illumination as u32)) {
                Some(result) => {
                    let _ = serial.send(&result);
                },
                None => {}
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

