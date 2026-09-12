use tokio::time::Duration;

pub const USBIP_TCP_PORT: u16 = 3240;
pub const DEFAULT_SERVER_PORT: u16 = 3241;
pub const DEFAULT_CLIENT_PORT: u16 = 3242;
pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_millis(1000);
pub const SETTLE_TRIES: u32 = 25;
pub const SETTLE_WAIT: Duration = Duration::from_millis(100);
pub const MAX_FAILED_PINGS: u8 = 5;
pub const MIN_RETRY_INTERVAL: Duration = Duration::from_millis(250);
pub const MAX_RETRY_INTERVAL: Duration = Duration::from_secs(64);
pub const MAX_RECONNECT_ATTEMPTS: u8 = 100;
