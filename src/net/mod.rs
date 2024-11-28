mod bupt;
mod provisioning;

use anyhow::{bail, Result};
use esp_idf_hal::{delay, modem::Modem};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    log::set_target_level,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use std::fmt;

fn connect_wifi_with_config(
    esp_wifi: &mut EspWifi<'static>,
    config: NetConfig,
    sys_loop: EspSystemEventLoop,
) -> Result<()> {
    let mut bupt_account = None;
    let (auth_method, ssid, pass) = match config {
        NetConfig::BuptPortal(account) => {
            bupt_account = Some(account);
            (AuthMethod::None, "BUPT-portal".to_string(), String::new())
        }
        NetConfig::NormalWifi(wifi) => (AuthMethod::WPA2Personal, wifi.ssid, wifi.password),
    };

    #[cfg(feature = "random_mac")]
    {
        use esp_idf_svc::wifi::WifiDeviceId;
        let mac = generate_random_mac();
        log::info!("Generated random MAC: {:02X?}", mac);
        esp_wifi.set_mac(WifiDeviceId::Sta, mac)?;
        log::info!(
            "Set MAC address to {:02X?}",
            esp_wifi.get_mac(WifiDeviceId::Sta)?
        );
    }

    let mut wifi = BlockingWifi::wrap(esp_wifi, sys_loop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: heapless::String::<32>::from_iter(ssid.chars()),
        password: heapless::String::<64>::from_iter(pass.chars()),
        channel: None,
        auth_method,
        ..Default::default()
    }))?;

    log::info!("Starting wifi...");
    wifi.start()?;
    log::info!("Connecting wifi {}...", ssid);
    let delay: delay::Delay = Default::default();

    for retry in 0..10 {
        match wifi.connect() {
            Ok(_) => break,
            Err(e) => {
                log::warn!(
                    "Failed to connect wifi: {}, will retry after 10 seconds...",
                    e
                );
            }
        }
        delay.delay_ms(1000 * 10);
        if retry == 9 {
            log::error!("Retry limit exceeded");
            bail!("Failed to connect to wifi");
        } else {
            log::info!("Retrying...");
        }
    }

    log::info!("Waiting for DHCP lease...");
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("Wifi DHCP info: {:?}", ip_info);

    if let Some(account) = bupt_account {
        for retry in 0..10 {
            match bupt::login(&account) {
                Ok(_) => break,
                Err(e) => {
                    log::warn!(
                        "Failed to login to BUPT-portal: {}, will retry after 10 seconds...",
                        e
                    );
                }
            }
            delay.delay_ms(1000 * 10);
            if retry == 9 {
                log::error!("Retry limit exceeded");
                bail!("Failed to login to BUPT-portal");
            } else {
                log::info!("Retrying...");
            }
        }
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Wifi {
    ssid: String,
    password: String,
}

impl fmt::Debug for Wifi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let password_length = self.password.len();
        let hidden_password = "*".repeat(password_length);
        f.debug_struct("Wifi")
            .field("ssid", &self.ssid)
            .field("password", &hidden_password)
            .finish()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
enum NetConfig {
    BuptPortal(bupt::BuptAccount),
    NormalWifi(Wifi),
}

#[cfg(feature = "random_mac")]
pub fn generate_random_mac() -> [u8; 6] {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut mac = [0u8; 6];
    rng.fill(&mut mac);
    mac[0] &= 0xFE;
    mac[0] &= 0xFD;
    mac
}

pub struct NetManager {
    pub wifi: EspWifi<'static>,
    pub sysloop: EspSystemEventLoop,
}

impl NetManager {
    pub fn new(modem: Modem) -> anyhow::Result<Self> {
        let sysloop = EspSystemEventLoop::take().unwrap();
        let nvs = crate::nvs::nvs();
        let esp_wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs))?;
        Ok(Self {
            wifi: esp_wifi,
            sysloop
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        set_target_level("wifi", log::LevelFilter::Warn)?;
        set_target_level("wifi_init", log::LevelFilter::Warn)?;

        #[cfg(feature = "clean_nvs")]
        crate::nvs::remove::<NetConfig>()?;

        match crate::nvs::load::<NetConfig>()? {
            Some(config) => {
                log::info!("Loaded NetConfig: {:?}", &config);
                connect_wifi_with_config(
                    &mut self.wifi,
                    config,
                    self.sysloop.clone(),
                )
            }
            None => {
                let p = provisioning::Provisioner::new(
                    &mut self.wifi,
                    self.sysloop.clone(),
                )?;
                p.wait();
                Ok(())
            }
        }
}

}
