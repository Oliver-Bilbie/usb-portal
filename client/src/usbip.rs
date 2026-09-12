use usb_portal_lib::prelude::*;

pub async fn fetch_connected() -> Result<Vec<ConnectedDevice>, String> {
    let body = usbip::port().await?;
    Ok(read_connected_devices(&body))
}

pub async fn wait_until_connected(bus_id: &str) -> bool {
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

pub async fn wait_until_detached(vhci_port: &str) -> bool {
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
