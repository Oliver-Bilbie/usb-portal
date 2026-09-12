use crate::helpers::attach_device;
use crate::state::State;
use futures::future::join_all;
use log::*;
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use usb_portal_lib::prelude::*;

struct ReconnectStatus {
    attempts: u8,
    delay: Duration,
    retry_time: Instant,
}

impl ReconnectStatus {
    fn new() -> ReconnectStatus {
        ReconnectStatus {
            attempts: 0,
            delay: MIN_RETRY_INTERVAL,
            retry_time: Instant::now(),
        }
    }
}

pub struct Reconnector {
    thread: Option<JoinHandle<()>>,
    token: CancellationToken,
}

impl Reconnector {
    pub fn init(state: Arc<State>) -> Reconnector {
        debug!("Starting reconnector thread");
        let token = CancellationToken::new();
        let thread = Some(tokio::spawn({
            let d = Arc::new(Mutex::new(HashMap::new()));
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
                            next_start = tokio::time::Instant::now() + MIN_RETRY_INTERVAL;
                            do_reconnect(&d, &s).await;
                        }
                    };
                }
            }
        }));
        Reconnector { thread, token }
    }

    pub async fn shutdown(&mut self) {
        debug!("Gracefully stopping the reconnector thread");
        self.token.cancel();
        if let Some(thread) = self.thread.take() {
            _ = thread.await;
        }
    }
}

impl Drop for Reconnector {
    fn drop(&mut self) {
        // Async drop is not yet supported in stable Rust, so we cannot await the termination of the
        // thread. Use the shutdown method instead.
        if !self.token.is_cancelled() {
            warn!("The reconnector thread was not terminated gracefully");
            self.token.cancel();
        }
    }
}

async fn do_reconnect(
    reconnect_status: &Arc<Mutex<HashMap<MissingDevice, ReconnectStatus>>>,
    state: &Arc<State>,
) {
    let missing = state.get_missing_devices();
    update_devices(missing, reconnect_status).await;
    reconnect_devices(reconnect_status, state).await;
}

async fn update_devices(
    missing_devices: HashSet<MissingDevice>,
    reconnect_status_mtx: &Arc<Mutex<HashMap<MissingDevice, ReconnectStatus>>>,
) {
    let mut reconnect_status = reconnect_status_mtx.lock().await;
    // Stop requesting any non-missing devices
    let to_drop: Vec<MissingDevice> = reconnect_status
        .keys()
        .filter(|d| !missing_devices.contains(d))
        .cloned()
        .collect();
    for d in to_drop {
        reconnect_status.remove(&d);
    }
    // Request any new missing devices
    for d in missing_devices {
        reconnect_status.entry(d).or_insert(ReconnectStatus::new());
    }
}

async fn reconnect_devices(
    reconnect_status: &Arc<Mutex<HashMap<MissingDevice, ReconnectStatus>>>,
    state: &Arc<State>,
) {
    let now = Instant::now();
    let to_retry: Vec<MissingDevice> = {
        let devices = reconnect_status.lock().await;
        devices
            .iter()
            .filter_map(|(d, v)| {
                if v.retry_time <= now {
                    return Some(d.clone());
                }
                None
            })
            .collect()
    };

    join_all(to_retry.into_iter().map(|d| async move {
        if let Some(s) = state.get_server(&d.server_ip) {
            debug!("Attempting to reconnect to {} on {}", d.device, d.server_ip);
            match attach_device(&d.server_ip, s.http_port, &d.device.bus_id, true, &state).await {
                true => {
                    info!("Connection was restored to {} on {}", d.device, d.server_ip);
                    state.drop_missing_device(&d);
                    reconnect_status.lock().await.remove(&d);
                }
                false => {
                    debug!("Failed to reconnect to {} on {}", d.device, d.server_ip);
                    let mut rs = reconnect_status.lock().await;
                    if let Some(status) = rs.get_mut(&d) {
                        if status.attempts < MAX_RECONNECT_ATTEMPTS {
                            status.attempts = status.attempts.saturating_add(1);
                            status.delay = MAX_RETRY_INTERVAL.min(status.delay * 2);
                            status.retry_time = now + status.delay;
                        } else {
                            debug!(
                                "Connection has timed out for {} on {}",
                                d.device, d.server_ip
                            );
                            rs.remove(&d);
                        }
                    };
                }
            }
        }
    }))
    .await;
}
