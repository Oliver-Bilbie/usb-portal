use crate::{helpers::ping, state::State};
use futures::future::join_all;
use log::*;
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::Arc,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use usb_portal_lib::prelude::*;

pub struct HealthCheck {
    thread: Option<JoinHandle<()>>,
    token: CancellationToken,
}

impl HealthCheck {
    pub fn init(state: Arc<State>) -> HealthCheck {
        debug!("Starting health check thread");
        let token = CancellationToken::new();
        let thread = Some(tokio::spawn({
            let s = state.clone();
            let t = token.clone();
            let mut next_start = tokio::time::Instant::now();
            async move {
                loop {
                    tokio::select! {
                        _ = t.cancelled() => {
                            return;
                        },
                        _ = tokio::time::sleep_until(next_start) => {
                            next_start = tokio::time::Instant::now() + HEALTH_CHECK_INTERVAL;
                            do_health_check(s.clone()).await;
                        }
                    };
                }
            }
        }));
        HealthCheck { thread, token }
    }

    pub async fn shutdown(&mut self) {
        debug!("Gracefully stopping the health check thread");
        self.token.cancel();
        if let Some(thread) = self.thread.take() {
            _ = thread.await;
        }
    }
}

impl Drop for HealthCheck {
    fn drop(&mut self) {
        // Async drop is not yet supported in stable Rust, so we cannot await the termination of the
        // thread. Use the shutdown method instead.
        if !self.token.is_cancelled() {
            warn!("The health check thread was not terminated gracefully");
            self.token.cancel();
        }
    }
}

async fn do_health_check(state: Arc<State>) {
    let missing_servers = update_server_health(&state).await;
    let expected_devices = state.get_connected_devices();

    update_missing_devices(&expected_devices, &missing_servers, &state).await;
}

async fn update_server_health(state: &Arc<State>) -> HashMap<Ipv4Addr, Server> {
    let all_servers: HashSet<Server> = state
        .get_servers_list()
        .into_iter()
        .chain(state.get_missing_servers().into_iter())
        .collect();

    let status = join_all(all_servers.into_iter().map(|s| async move {
        if ping(&s.ip, s.http_port).await {
            state.add_server(s.clone());
            state.reset_failed_pings(&s);
        } else {
            let fail_count = state.increment_failed_pings(s.clone());
            if fail_count >= MAX_FAILED_PINGS {
                if fail_count == MAX_FAILED_PINGS {
                    info!("Lost connection to server {}", s.hostname);
                    state.drop_server(&s.ip);
                }
                return Err((s.ip, s));
            }
        }
        Ok((s.ip, s))
    }))
    .await;

    status.into_iter().filter_map(|s| s.err()).collect()
}

async fn update_missing_devices(
    expected_devices: &Vec<ConnectedDevice>,
    missing_servers: &HashMap<Ipv4Addr, Server>,
    state: &Arc<State>,
) {
    join_all(
        expected_devices
            .iter()
            .filter(|d| missing_servers.contains_key(&d.server_ip))
            .map(|d| async {
                info!("Connection to {} was lost", d.device);
                state.drop_connected_device(d);
                state.add_missing_device(d);
                let _ = usbip::detach(&d.vhci_port).await;
            }),
    )
    .await;
}
