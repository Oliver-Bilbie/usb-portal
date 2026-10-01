use crate::{
    helpers::{fetch_available, fetch_connected},
    notification::Notification,
    state::State,
};
use askama::Template;
use log::*;
use std::{collections::HashSet, net::Ipv4Addr, sync::Arc};
use usb_portal_lib::types::*;

#[derive(Template)]
#[template(path = "notifications.html")]
struct NotificationsTempl {
    notifications: Vec<Notification>,
}

pub fn notifications(state: &Arc<State>) -> String {
    let notifications = state.get_notifications();
    NotificationsTempl { notifications }.render().unwrap()
}

#[derive(Template)]
#[template(path = "connected.html")]
pub struct ConnectedTempl {
    pub devices: Vec<ConnectedDevice>,
}

pub fn connected(state: &Arc<State>) -> String {
    let devices = state.get_connected_devices();
    ConnectedTempl { devices }.render().unwrap()
}

#[derive(Template)]
#[template(path = "available.html")]
pub struct AvailableTempl {
    pub devices: Vec<AvailableDevice>,
}

pub async fn available(state: &Arc<State>) -> String {
    let connected: HashSet<(Ipv4Addr, String, String)> = fetch_connected()
        .await
        .unwrap_or(vec![])
        .into_iter()
        .map(|d| (d.server_ip, d.device.vendor_id, d.device.product_id))
        .collect();
    let mut available = Vec::<AvailableDevice>::new();

    for server in state.get_servers_list() {
        debug!("Scanning server: {}", server.ip);
        for device in fetch_available(&server.ip, server.http_port)
            .await
            .unwrap_or(vec![])
        {
            let key = (
                server.ip.clone(),
                device.vendor_id.clone(),
                device.product_id.clone(),
            );
            if !connected.contains(&key) {
                available.push(AvailableDevice {
                    device,
                    server: server.clone(),
                });
            }
        }
    }
    AvailableTempl { devices: available }.render().unwrap()
}

#[derive(Template)]
#[template(path = "body.html")]
pub struct BodyTempl<'a> {
    pub notifications: &'a str,
    pub connected: &'a str,
    pub available: &'a str,
}

pub async fn body(state: &Arc<State>) -> String {
    let notifications = &notifications(&state);
    let connected = &connected(&state);
    let available = &available(state).await;
    BodyTempl {
        notifications,
        connected,
        available,
    }
    .render()
    .unwrap()
}
