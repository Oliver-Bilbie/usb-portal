use std::fmt;
use std::net::Ipv4Addr;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Server {
    pub hostname: String,
    pub ip: Ipv4Addr,
    pub http_port: u16,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct Device {
    pub vendor: String,
    pub product: String,
    pub vendor_id: String,
    pub product_id: String,
    pub bus_id: String,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — {}", self.vendor, self.product)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct AvailableDevice {
    pub device: Device,
    pub server: Server,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConnectedDevice {
    pub device: Device,
    pub server_ip: Ipv4Addr,
    pub vhci_port: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MissingDevice {
    pub device: Device,
    pub server_ip: Ipv4Addr,
}
