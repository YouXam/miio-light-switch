use std::collections::HashMap;
use crate::parser::Value;
use crate::serial::Property;

pub struct Storage {
    pub siid: u32,
    pub piid: u32,
    pub value: Value,
}

pub struct IoTFramework {
    properties: HashMap<(u32, u32), Storage>,
    callbacks: HashMap<(u32, u32), Box<dyn FnMut(&Value)>>,
}

impl IoTFramework {
    pub fn new() -> Self {
        IoTFramework {
            properties: HashMap::new(),
            callbacks: HashMap::new(),
        }
    }

    pub fn register_property(&mut self, siid: u32, piid: u32, value: Value, callback: impl FnMut(&Value) + 'static) {
        let prop = Storage { siid, piid, value };
        self.properties.insert((siid, piid), prop);
        self.callbacks.insert((siid, piid), Box::new(callback));
    }

    pub fn on_get_properties(&self, props: Vec<Property>) -> String {
        let mut response = Vec::new();

        for prop in props {
            let key = (prop.siid, prop.piid);
            if let Some(p) = self.properties.get(&key) {
                let code = 0;  // 操作成功
                let value_str = match &p.value {
                    Value::Integer(val) => val.to_string(),
                    Value::Boolean(val) => val.to_string(),
                    Value::String(val) => val.clone(),
                };
                response.push(format!("{} {} {} {}", p.siid, p.piid, code, value_str));
            } else {
                response.push(format!("{} {} -4003", prop.siid, prop.piid)); // 属性不存在
            }
        }

        format!("result {}", response.join(" "))
    }

    pub fn on_set_properties(&mut self, props: Vec<Property>) -> Vec<String> {
        let mut response = Vec::new();
        let mut result = Vec::new();

        for prop in props {
            let key = (prop.siid, prop.piid);
            match prop.value {
                Some(value) => {
                    if let Some(p_existing) = self.properties.get_mut(&key) {
                        p_existing.value = value;
                        response.push(format!("{} {} 0", p_existing.siid, p_existing.piid));
                        result.push(format!("{} {} {}", p_existing.siid, p_existing.piid, p_existing.value));
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

        vec![
            format!("result {}", response.join(" ")),
            format!("properties_changed {}", result.join(" "))
        ]
    }

    pub fn set_property(&mut self, siid: u32, piid: u32, value: Value) -> Option<String> {
        let key = (siid, piid);
        if let Some(prop) = self.properties.get_mut(&key) {
            if prop.value != value {
                prop.value = value.clone();
                if let Some(callback) = self.callbacks.get_mut(&key) {
                    callback(&prop.value);
                }
                Some(format!("properties_changed {} {} {}", siid, piid, &prop.value))
            } else {
                None
            }
        } else {
            None
        }
    }
}
