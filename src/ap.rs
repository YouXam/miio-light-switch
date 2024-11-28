use embedded_svc::http::client::Client;
use esp_idf_svc::http::{client::{Configuration, EspHttpConnection}, Method};

#[derive(Debug, serde::Deserialize)]
pub struct Ap {
    pub sta_count: String,
    pub ap_name: String
}

#[derive(Debug, serde::Deserialize)]
pub struct Status {
    pub ap: Ap,
}

pub fn status() -> anyhow::Result<Status> {
    log::info!("[-] Getting AP status...");
    let connection = EspHttpConnection::new(&Configuration::default())?;
    let mut client = Client::wrap(connection);
    let request = client.request(Method::Post, "http://buptnet.icu/api/wireless/diag", &[])?;
    let mut response = request.submit()?;
    let status = response.status();
    log::info!("    Response code: {}", status);
    match status {
        200..=299 => {
            let mut data: Vec<u8> = Vec::new();
            loop {
                let mut buffer = [0u8; 1024];
                if let Ok(size) = response.read(&mut buffer) {
                    if size == 0 {
                        break;
                    }
                    data.extend_from_slice(&buffer[..size]);
                } else {
                    break;
                }
            }
            let body = String::from_utf8(data)
                .map_err(|e| anyhow::anyhow!("failed to parse response body: {}", e))?;
            let status: Status = serde_json::from_str(&body)?;
            Ok(status)
        },
        _ => {
            log::error!("[!] Unexpected status code: {}", status);
            Err(anyhow::anyhow!("Unexpected status code: {}", status))
        }
    }
}