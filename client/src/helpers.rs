use crate::state::State;
use log::*;
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use usb_portal_lib::prelude::*;

pub async fn ping(ip: &Ipv4Addr, http_port: u16) -> bool {
    server_get(ip, http_port, "/ping", 250).await.is_ok()
}

pub async fn server_get(
    ip: &Ipv4Addr,
    http_port: u16,
    path: &str,
    timeout_ms: u64,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("http://{}:{}{}", ip, http_port, path);
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(if body.is_empty() {
            status.to_string()
        } else {
            body
        })
    }
}

pub async fn fetch_available(ip: &Ipv4Addr, http_port: u16) -> Result<Vec<Device>, String> {
    let body = server_get(ip, http_port, "/list", 1000).await?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

pub fn server_http_port(state: &State, ip: &Ipv4Addr) -> u16 {
    match state.get_server(ip) {
        Some(s) => s.http_port,
        None => {
            warn!(
                "No server data found for {}; falling back to default port.",
                ip
            );
            DEFAULT_SERVER_PORT
        }
    }
}

pub async fn discover_servers(state: &Arc<State>) {
    for mut s in browse(Duration::from_secs(1)).await {
        if state.get_server(&s.ip).is_none() {
            if let Ok(hostname) = server_get(&s.ip, s.http_port, "/hostname", 500).await {
                s.hostname = hostname;
            }
            debug!(
                "Discovered server {} at {}:{}",
                s.hostname, s.ip, s.http_port
            );
            state.add_server(s);
        }
    }
}

pub async fn add_manual_server(ip: Ipv4Addr, http_port: u16, state: &Arc<State>) -> bool {
    if ping(&ip, http_port).await {
        state.add_server(Server {
            hostname: ip.to_string(),
            ip,
            http_port,
        });
        return true;
    }
    state.push_notification(
        Level::Info,
        format!("No response from server {}:{}", ip, http_port),
    );
    false
}

pub async fn attach_device(
    ip: &Ipv4Addr,
    http_port: u16,
    bus_id: &str,
    silent: bool,
    state: &Arc<State>,
) -> bool {
    if let Err(msg) = server_get(ip, http_port, &format!("/bind/{}", bus_id), 5000).await {
        if !msg.contains("already bound to usbip-host") {
            if !silent {
                state.push_notification(
                    Level::Warn,
                    format!("Unable to bind {} on server: {}", bus_id, msg),
                );
            }
            return false;
        }
    }
    let task = UsbipTask::Attach(AttachArgs { ip, bus_id });
    if let Err(msg) = task.run().await {
        if !silent {
            state.push_notification(
                Level::Warn,
                format!("Bound on server but attach failed ({}): {}", bus_id, msg),
            );
        }
        if let Err(unbind_msg) =
            server_get(ip, http_port, &format!("/unbind/{}", bus_id), 5000).await
        {
            if !silent {
                state.push_notification(
                    Level::Warn,
                    format!(
                        "Unable to detach device on server ({}): {}",
                        bus_id, unbind_msg
                    ),
                );
            }
        }
        return false;
    }
    if !wait_until_connected(&bus_id).await {
        if !silent {
            state.push_notification(
                Level::Warn,
                format!(
                    "Attempt to connect to device at {} on bus {} timed out",
                    ip, bus_id
                ),
            );
        }
        if let Err(unbind_msg) =
            server_get(ip, http_port, &format!("/unbind/{}", bus_id), 5000).await
        {
            if !silent {
                state.push_notification(
                    Level::Warn,
                    format!(
                        "Unable to detach device on server ({}): {}",
                        bus_id, unbind_msg
                    ),
                );
            }
        }
        return false;
    }
    let connected = fetch_connected().await.unwrap_or_default();
    let prev_connected = state.get_connected_devices();
    for d in connected {
        if d.server_ip == *ip && d.device.bus_id == *bus_id && !prev_connected.contains(&d) {
            debug!("Added connection to {}", d.device);
            state.add_connected_device(d);
            break;
        }
    }
    true
}

pub async fn detach_device(
    ip: &Ipv4Addr,
    bus_id: &str,
    vhci_port: &str,
    state: &Arc<State>,
) -> bool {
    let http_port = match state.get_server(ip) {
        Some(s) => s.http_port,
        None => {
            warn!(
                "No server record was found for {}. Trying default server port.",
                ip
            );
            DEFAULT_SERVER_PORT
        }
    };
    if let Err(msg) = server_get(ip, http_port, &format!("/unbind/{}", bus_id), 5000).await {
        state.push_notification(
            Level::Warn,
            format!("unable to unbind {} on server: {}", bus_id, msg),
        );
        return false;
    }
    let task = UsbipTask::Detach(DetachArgs { vhci_port });
    if let Err(msg) = task.run().await {
        state.push_notification(
            Level::Warn,
            format!("unable to detach vhci port {}: {}", vhci_port, msg),
        );
        return false;
    } else {
        if !wait_until_detached(vhci_port).await {
            state.push_notification(
                Level::Warn,
                format!(
                    "Attempt to connect to device at {} on bus {} timed out",
                    ip, bus_id
                ),
            );
            return false;
        }
        let connected = fetch_connected().await.unwrap_or_default();
        let prev_connected = state.get_connected_devices();
        for d in prev_connected {
            if d.server_ip == *ip && d.device.bus_id == bus_id && !connected.contains(&d) {
                debug!("Dropped connection to {}", d.device);
                state.drop_connected_device(&d);
                break;
            }
        }
    }
    true
}

pub async fn release_all(state: Arc<State>) {
    let devices = match fetch_connected().await {
        Ok(v) => v,
        Err(msg) => {
            error!("unable to release devices\n{}", msg);
            return;
        }
    };
    for d in devices {
        let task = UsbipTask::Detach(DetachArgs {
            vhci_port: &d.vhci_port,
        });
        match task.run().await {
            Ok(_) => {
                info!("Released device {}", d.device);
            }
            Err(msg) => {
                error!("Unable to detach device {}\n{}", d.device, msg);
            }
        }
        if let Err(msg) = server_get(
            &d.server_ip,
            server_http_port(&state, &d.server_ip),
            &format!("/unbind/{}", d.device.bus_id),
            5000,
        )
        .await
        {
            if !msg.contains("device is not bound to usbip-host driver") {
                error!("unable to unbind device {}\n{}", d.device, msg)
            }
        }
    }
}

pub async fn fetch_connected() -> Result<Vec<ConnectedDevice>, String> {
    let task = UsbipTask::Port;
    let body = task.run().await?;
    Ok(read_connected_devices(&body))
}

async fn wait_until_connected(bus_id: &str) -> bool {
    for _ in 0..SETTLE_TRIES {
        if let Ok(devs) = fetch_connected().await
            && devs.iter().any(|d| d.device.bus_id == bus_id)
        {
            return true;
        }
        tokio::time::sleep(SETTLE_WAIT).await;
    }
    false
}

async fn wait_until_detached(vhci_port: &str) -> bool {
    for _ in 0..SETTLE_TRIES {
        if let Ok(devs) = fetch_connected().await
            && !devs.iter().any(|d| d.vhci_port == vhci_port)
        {
            return true;
        }
        tokio::time::sleep(SETTLE_WAIT).await;
    }
    false
}
