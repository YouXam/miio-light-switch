
use esp_idf_hal::gpio::{AnyIOPin, InputPin, OutputPin};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::uart::{self, Uart};
use esp_idf_hal::{delay, prelude::*};
use std::fmt::Write;
use std::time::{Duration, Instant};

use crate::parser::{parse, Value};

fn readline(uart: &uart::UartDriver) -> anyhow::Result<String> {
    let mut buf = [0u8; 1024];
    let mut s = String::new();
    loop {
        let cnt = uart.read(&mut buf, delay::TICK_RATE_HZ / 100)?;
        for i in 0..cnt {
            if buf[i] == 0 {
                break;
            }
            if buf[i] == '\r' as u8 || buf[i] == '\n' as u8 {
                return Ok(s);
            }
            write!(s, "{}", buf[i] as char)?;
        }
    }
}

fn readline_timeout(uart: &uart::UartDriver, timeout_ms: u64) -> anyhow::Result<String> {
    let mut buf = [0u8; 1024];
    let mut s = String::new();

    let timeout = Duration::from_millis(timeout_ms);
    let start_time = Instant::now();

    loop {
        if start_time.elapsed() >= timeout {
            return Err(anyhow::anyhow!("timeout"));
        }

        let cnt = uart.read(&mut buf, delay::TICK_RATE_HZ / 100)?;
        for i in 0..cnt {
            if buf[i] == 0 {
                break;
            }
            if buf[i] == b'\r' || buf[i] == b'\n' {
                return Ok(s);
            }
            s.push(buf[i] as char);
        }
    }
}

fn writeline(uart: &mut uart::UartDriver, s: &str) -> anyhow::Result<()> {
    write!(uart, "{}\r", s)?;
    Ok(())
}

pub struct Serial {
    uart: uart::UartDriver<'static>,
    model: Option<&'static str>,
    version: Option<&'static str>,
    pid: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub siid: u32,
    pub piid: u32,
    pub value: Option<Value>
}

#[derive(Debug)]
pub enum Event {
    SetProperties(Vec<Property>),
    GetProperties(Vec<Property>),
    Unknown
}

// get_properties <siid> <piid> ... <siid> <piid>
fn parse_get_properties(input: &str) -> anyhow::Result<Vec<Property>> {
    let mut properties = Vec::new();
    let mut iter = parse(input.trim_start_matches("get_properties ")).into_iter();
    while let Some(siid) = iter.next() {
        let siid = match siid {
            Value::Integer(siid) => siid,
            _ => return Err(anyhow::anyhow!("Expected integer, got {:?}", siid))
        };
        let piid = match iter.next() {
            Some(Value::Integer(piid)) => piid,
            Some(other) => return Err(anyhow::anyhow!("Expected integer, got {:?}", other)),
            None => return Err(anyhow::anyhow!("Expected integer, got None"))
        };
        properties.push(Property {
            siid: siid,
            piid: piid,
            value: None
        });
    }
    Ok(properties)
}

// set_properties <siid> <piid> <value> ... <siid> <piid> <value>
fn parse_set_properties(input: &str) -> anyhow::Result<Vec<Property>> {
    let mut properties = Vec::new();
    let mut iter = parse(input.trim_start_matches("set_properties ")).into_iter();
    while let Some(siid) = iter.next() {
        let siid = match siid {
            Value::Integer(siid) => siid,
            _ => return Err(anyhow::anyhow!("Expected integer, got {:?}", siid))
        };
        let piid = match iter.next() {
            Some(Value::Integer(piid)) => piid,
            Some(other) => return Err(anyhow::anyhow!("Expected integer, got {:?}", other)),
            None => return Err(anyhow::anyhow!("Expected integer, got None"))
        };
        match iter.next() {
            Some(value) => {
                properties.push(Property {
                    siid: siid,
                    piid: piid,
                    value: Some(value)
                });
            },
            None => return Err(anyhow::anyhow!("Expected value, got None"))
        }
    }
    Ok(properties)
}

impl Serial {
    pub fn new(
        uart: impl Peripheral<P = impl Uart>,
        tx: impl Peripheral<P = impl OutputPin>,
        rx: impl Peripheral<P = impl InputPin>,
    ) -> Self {
        let config = uart::config::Config::default().baudrate(Hertz(115_200));

        let uart: uart::UartDriver<'static> = unsafe {
            core::mem::transmute(
                uart::UartDriver::new(
                    uart,
                    tx,
                    rx,
                    Option::<AnyIOPin>::None,
                    Option::<AnyIOPin>::None,
                    &config,
                )
                .unwrap(),
            )
        };

        Serial {
            uart: uart,
            model: None,
            version: None,
            pid: None,
        }
    }

    pub fn send(&mut self, message: &str) -> anyhow::Result<String> {
        log::info!("[+] <- {}", message);
        writeline(&mut self.uart, message)?;
        let response = readline_timeout(&self.uart, 500)?;
        log::info!("    -> {}", response);
        Ok(response)
    }

    pub fn model(&mut self, model: &'static str) -> anyhow::Result<()> {
        self.model = Some(model);
        let response = self.send(&format!("model {}", model))?;
        if response == "ok" {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: model {}", response, model))
        }
    }

    pub fn result(&mut self, result: String) -> anyhow::Result<()> {
        let response = self.send(&format!("result {}", result))?;
        if response == "ok" {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: result {}", response, result))
        }
    }

    pub fn version(&mut self, version: &'static str, pid: &'static str) -> anyhow::Result<()> {
        self.version = Some(version);
        self.pid = Some(pid);
        let response = self.send(&format!("mcu_version {}", version))?;
        if response != "ok" {
            return Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: mcu_version {}", response, version))
        }
        self.send(&format!("ble_config dump"))?;
        let response = self.send(&format!("ble_config set {} {}", pid, version))?;
        if response != "ok" {
            return Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: ble_config set {} {}", response, pid, version))
        }
        Ok(())
    }

    pub fn get_down(&mut self) -> anyhow::Result<Option<Event>> {
        // log::info!("[+] <- {}", "get_down");
        writeline(&mut self.uart, "get_down")?;
        let response = readline_timeout(&self.uart, 1000)?;
        // log::info!("    -> {}", response);
        if response == "down none" {
            Ok(None)
        } else if response.starts_with("down ") {
            let command = response.trim_start_matches("down ");
            log::info!("[>] {}", command);
            if command.starts_with("set_properties") {
                let properties = parse_set_properties(command)?;
                Ok(Some(Event::SetProperties(properties)))
            } else if command.starts_with("get_properties") {
                let properties = parse_get_properties(command)?;
                Ok(Some(Event::GetProperties(properties)))
            } else if command.starts_with("MIIO_net_change ") {
                Ok(None)
            } else if command.starts_with("miIO.get_powermode") {
                self.send("result 1")?;
                Ok(None)
            } else {
                log::warn!("Unknown command: {}", command);
                Ok(Some(Event::Unknown))
            }
        } else {
            log::error!("Unexpected response: {}\n\tCommand: get_down", response);
            Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: get_down", response))
        }
    }

    #[allow(dead_code)]
    pub fn restore(&mut self) -> anyhow::Result<()> {
        let response = self.send("restore")?;
        if response != "ok" {
            return Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: restore", response))
        }
        let response2 = self.send("reboot")?;
        if response2 == "ok" {
            std::thread::sleep(Duration::from_millis(1000));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unexpected response: {}\n\tCommand: reboot", response2))
        }
    }
}
