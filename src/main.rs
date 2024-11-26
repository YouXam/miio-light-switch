use std::time::Duration;
use esp_idf_hal::prelude::Peripherals;
use parser::Value;

mod net;
mod nvs;
mod serial;
mod parser;
mod miio;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    

    let peripherals = Peripherals::take().unwrap();

    // net::connect(Some(peripherals.modem))?.stop()?;

    let pins = peripherals.pins;

    let mut serial = crate::serial::Serial::new(
        peripherals.uart1,
        pins.gpio12,
        pins.gpio11
    );

    #[cfg(feature = "restore")]
    serial.restore()?;

    serial.model("csbupt.switch.smsw")?;
    let _ = serial.version("0001", "24351");

    let mut miio = crate::miio::IoTFramework::new();

    // 企业标志
    miio.register_property(1, 1, Value::String("YouXam".to_string()), |_| {});
    // 产品模型
    miio.register_property(1, 2, Value::String("csbupt.switch.smsw".to_string()), |_| {});
    // 设备ID
    miio.register_property(1, 3, Value::String("0001".to_string()), |_| {});
    // 固件版本号
    miio.register_property(1, 4, Value::String("0001".to_string()), |_| {});

    // 开关
    miio.register_property(2, 1, Value::Boolean(false), |e| {
        println!("Switch: {:?}", e);
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


    loop {
        if let Ok(Some(event)) = serial.get_down() {
            match event {
                crate::serial::Event::SetProperties(props) => {
                    let response = miio.on_set_properties(props);
                    for r in response {
                        serial.send(&r)?;
                    }
                }
                crate::serial::Event::GetProperties(props) => {
                    let response = miio.on_get_properties(props);
                    serial.send(&response)?;
                }
                crate::serial::Event::Unknown => {}
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
