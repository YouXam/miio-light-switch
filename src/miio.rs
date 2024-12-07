use std::collections::HashMap;
use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::uart::Uart;

use crate::parser::Value;
use crate::serial::{Property, Serial};

pub struct Storage {
    pub siid: u32,
    pub piid: u32,
    pub value: Value,
}

pub struct IoTFramework {
    properties: HashMap<(u32, u32), Storage>,
    callbacks: HashMap<(u32, u32), Box<dyn FnMut(&Value)>>,
    serial: Serial,
    model: &'static str,
    version: &'static str,
    pid: &'static str,
    siid: u32,
    piid: u32,
}

impl IoTFramework {
    pub fn new(
        uart: impl Peripheral<P = impl Uart>,
        tx: impl Peripheral<P = impl OutputPin>,
        rx: impl Peripheral<P = impl InputPin>,
        model: &'static str,
        version: &'static str,
        pid: &'static str,
    ) -> anyhow::Result<Self> {
        let mut serial = Serial::new(uart, tx, rx);
        serial.model(model)?;
        let _ = serial.version(version, pid);

        Ok(IoTFramework {
            properties: HashMap::new(),
            callbacks: HashMap::new(),
            serial,
            model,
            version,
            pid,
            siid: 0,
            piid: 0,
        })
    }

    #[allow(dead_code)]
    pub fn restore(&mut self) -> anyhow::Result<()> {
        self.serial.restore()?;
        self.serial.model(self.model)?;
        let _ = self.serial.version(self.version, self.pid);
        Ok(())
    }

    pub fn callback(&mut self, siid: u32, piid: u32, callback: impl FnMut(&Value) + 'static) -> &mut Self {
        self.callbacks.insert((siid, piid), Box::new(callback));
        self
    }

    pub fn register<T: Into<Value>>(&mut self, siid: u32, piid: u32, value: T) -> &mut Self {
        let prop = Storage { siid, piid, value: value.into() };
        self.siid = siid;
        self.piid = piid;
        self.properties.insert((siid, piid), prop);
        self
    }

    pub fn load(&mut self) -> anyhow::Result<&mut Self> {
        if let Some(data) = crate::nvs::load_from::<Value>(&format!("{}.{}", self.siid, self.piid))? {
            self.set_property(self.siid, self.piid, data)?;
        }
        Ok(self)
    }

    pub fn registers<T: Into<Value>>(&mut self, values: Vec<(u32, u32, T)>) -> &mut Self {
        for (siid, piid, value) in values.into_iter() {
            self.register::<T>(siid, piid, value);
        }
        self
    }

    pub fn on(&mut self, callback: impl FnMut(&Value) + 'static) -> &mut Self {
        self.callback(self.siid, self.piid, callback)
    }


    pub fn on_get_properties(&self, props: Vec<Property>) -> String {
        let mut response = Vec::new();

        for prop in props {
            let key = (prop.siid, prop.piid);
            if let Some(p) = self.properties.get(&key) {
                let code = 0;  // 操作成功
                response.push(format!("{} {} {} {}", p.siid, p.piid, code, &p.value));
            } else {
                response.push(format!("{} {} -4003", prop.siid, prop.piid)); // 属性不存在
            }
        }

        format!("result {}", response.join(" "))
    }

    pub fn on_set_properties(&mut self, props: Vec<Property>) -> anyhow::Result<Vec<String>> {
        let mut response = Vec::new();
        let mut result = Vec::new();

        for prop in props {
            let key = (prop.siid, prop.piid);
            match prop.value {
                Some(value) => {
                    if let Some(p_existing) = self.properties.get_mut(&key) {
                        p_existing.value = value.clone();
                        response.push(format!("{} {} 0", p_existing.siid, p_existing.piid));
                        result.push(format!("{} {} {}", p_existing.siid, p_existing.piid, p_existing.value));
                        crate::nvs::save_to::<Value>(value, &format!("{}.{}", prop.siid, prop.piid))?;
                        if let Some(callback) = self.callbacks.get_mut(&key) {
                            callback(&p_existing.value);
                        }
                    } else {
                        response.push(format!("{} {} -4003", prop.siid, prop.piid));
                    }
                },
                None => {}
            }
        }

        Ok(vec![
            format!("result {}", response.join(" ")),
            format!("properties_changed {}", result.join(" "))
        ])
    }

    pub fn set_property(&mut self, siid: u32, piid: u32, value: Value) -> anyhow::Result<()> {
        let key = (siid, piid);
        if let Some(prop) = self.properties.get_mut(&key) {
            if prop.value != value {
                prop.value = value.clone();
                crate::nvs::save_to::<Value>(value, &format!("{}.{}", siid, piid))?;
                if let Some(callback) = self.callbacks.get_mut(&key) {
                    callback(&prop.value);
                }
                let event = format!("properties_changed {} {} {}", siid, piid, &prop.value);
                self.serial.send(&event)?;
            }
        }
        Ok(())
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        if let Ok(Some(event)) = self.serial.get_down() {
            match event {
                crate::serial::Event::SetProperties(props) => {
                    let response = self.on_set_properties(props)?;
                    for r in response {
                        let _ = self.serial.send(&r);
                    }
                }
                crate::serial::Event::GetProperties(props) => {
                    let response = self.on_get_properties(props);
                    let _ = self.serial.send(&response);
                }
                crate::serial::Event::Unknown => {}
            }
        }
        Ok(())
    }
}
