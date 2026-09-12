use crate::types::Server;

use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::time::{Duration, Instant};

pub const SERVICE_TYPE: &str = "_usb-portal._tcp.local.";

pub struct Advertisement {
    mdns: ServiceDaemon,
    fullname: String,
}

impl Drop for Advertisement {
    fn drop(&mut self) {
        let _ = self.mdns.unregister(&self.fullname);
        if let Ok(status) = self.mdns.shutdown() {
            let _ = status.recv_timeout(Duration::from_secs(1));
        }
    }
}

pub fn advertise(port: u16) -> Result<Advertisement, String> {
    let mdns = ServiceDaemon::new().map_err(|e| e.to_string())?;
    mdns.disable_interface(IfKind::IPv6)
        .map_err(|e| e.to_string())?;
    let instance = hostname();
    let host_name = format!("{}.local.", instance);
    let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host_name, "", port, None)
        .map_err(|e| e.to_string())?
        .enable_addr_auto();
    let fullname = info.get_fullname().to_string();
    mdns.register(info).map_err(|e| e.to_string())?;
    Ok(Advertisement { mdns, fullname })
}

pub async fn browse(timeout: Duration) -> Vec<Server> {
    tokio::task::spawn_blocking(move || browse_sync(timeout))
        .await
        .unwrap_or_default()
}

fn browse_sync(timeout: Duration) -> Vec<Server> {
    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let _ = mdns.disable_interface(IfKind::IPv6);
    let receiver = match mdns.browse(SERVICE_TYPE) {
        Ok(r) => r,
        Err(_) => {
            let _ = mdns.shutdown();
            return Vec::new();
        }
    };
    let deadline = Instant::now() + timeout;
    let mut found = Vec::new();
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if !info.is_valid() {
                    continue;
                }
                let host = info.host.trim_end_matches('.');
                let hostname = host.strip_suffix(".local").unwrap_or(host).to_string();
                for ip in info.get_addresses_v4() {
                    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
                        continue;
                    }
                    found.push(Server {
                        hostname: hostname.clone(),
                        ip,
                        http_port: info.port,
                    });
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = mdns.stop_browse(SERVICE_TYPE);
    if let Ok(status) = mdns.shutdown() {
        let _ = status.recv_timeout(Duration::from_secs(1));
    }
    found.sort();
    found.dedup();
    found
}

fn hostname() -> String {
    let name = std::fs::read_to_string("/etc/hostname")
        .ok()
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
        })
        .unwrap_or_default();
    let name = name.trim();
    let name = name.split('.').next().unwrap_or(name);
    if name.is_empty() {
        "usb-portal".to_string()
    } else {
        name.to_string()
    }
}
