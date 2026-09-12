use crate::types::{ConnectedDevice, Device};
use std::net::Ipv4Addr;

use log::*;

fn from_lines(line1: &str, line2: &str) -> Option<Device> {
    let bus_data = line1.strip_prefix("- busid ")?;
    let (bus_id, ids) = bus_data.split_once(" (")?;
    let ids = ids.strip_suffix(')')?;
    let (vendor_id, product_id) = ids.split_once(':')?;

    let (vendor, product) = {
        let (vendor, rest) = line2.split_once(" : ")?;
        let product = rest.rsplit_once(" (")?.0;
        (vendor.to_string(), product.to_string())
    };

    Some(Device {
        vendor,
        product,
        vendor_id: vendor_id.to_string(),
        product_id: product_id.to_string(),
        bus_id: bus_id.to_string(),
    })
}

pub fn read_devices(input: &str) -> Vec<Device> {
    let lines: Vec<&str> = input.lines().map(str::trim).collect();
    let mut devices = Vec::<Device>::with_capacity(lines.len() / 3);
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("- busid ")
            && let Some(next_line) = lines.get(i + 1)
            && let Some(d) = from_lines(line, next_line)
        {
            devices.push(d);
        }
    }
    devices
}

pub fn read_connected_devices(input: &str) -> Vec<ConnectedDevice> {
    let parse_line_1 = |line: &str| -> Option<String> {
        let rest = match line.strip_prefix("Port ") {
            Some(v) => v,
            None => {
                trace!("Line '{}' is not a connected device line 1", line);
                return None;
            }
        };
        let (vhci_port, _) = rest.split_once(':')?;
        Some(vhci_port.to_string())
    };

    let parse_line_2 = |line: &str| -> Option<(String, String, String, String)> {
        let (vendor, rest) = line.split_once(" : ")?;
        let (product, rest) = rest.split_once(" (")?;
        let (vendor_id, rest) = rest.split_once(':')?;
        let (product_id, _) = rest.split_once(')')?;
        Some((
            vendor.to_string(),
            product.to_string(),
            vendor_id.to_string(),
            product_id.to_string(),
        ))
    };

    let parse_line_3 = |line: &str| -> Option<(Ipv4Addr, String)> {
        let (_, rest) = line.split_once("usbip://")?;
        let (ip_str, rest) = rest.split_once(':')?;
        let (_, bus_id) = rest.split_once('/')?;
        let ip_parts: Vec<u8> = ip_str
            .splitn(4, '.')
            .filter_map(|x| x.parse::<u8>().ok())
            .collect();
        if ip_parts.len() != 4 {
            warn!("Failed to parse IP: {}", ip_str);
            return None;
        }
        let ip = Ipv4Addr::new(ip_parts[0], ip_parts[1], ip_parts[2], ip_parts[3]);
        Some((ip, bus_id.to_string()))
    };

    let lines: Vec<&str> = input.lines().map(str::trim).collect();
    let mut devices = Vec::new();
    let mut i = 0;
    while i + 2 < lines.len() {
        let vhci_port = match parse_line_1(lines[i]) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };
        let (vendor, product, vendor_id, product_id) = match parse_line_2(lines[i + 1]) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };
        let (server_ip, bus_id) = match parse_line_3(lines[i + 2]) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };
        devices.push(ConnectedDevice {
            device: Device {
                vendor,
                product,
                vendor_id,
                product_id,
                bus_id,
            },
            server_ip,
            vhci_port,
        });
        i += 3;
    }
    devices
}

pub fn read_available_devices(input: &str) -> Vec<Device> {
    let parse_line_1 = |line: &str| -> Option<String> {
        let (_, rest) = line.split_once("- busid ")?;
        let (bus_id, _) = rest.split_once(' ')?;
        Some(bus_id.to_string())
    };

    let parse_line_2 = |line: &str| -> Option<(String, String, String, String)> {
        let (vendor, rest) = line.split_once(" : ")?;
        let (product, rest) = rest.split_once(" (")?;
        let (vendor_id, rest) = rest.split_once(':')?;
        let (product_id, _) = rest.split_once(')')?;
        Some((
            vendor.to_string(),
            product.to_string(),
            vendor_id.to_string(),
            product_id.to_string(),
        ))
    };

    let lines: Vec<&str> = input.lines().map(str::trim).collect();
    let mut devices = Vec::new();
    let mut i = 0;
    while i + 2 < lines.len() {
        let bus_id = match parse_line_1(lines[i]) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };
        let (vendor, product, vendor_id, product_id) = match parse_line_2(lines[i + 1]) {
            Some(v) => v,
            None => {
                i += 1;
                continue;
            }
        };
        devices.push(Device {
            vendor,
            product,
            vendor_id,
            product_id,
            bus_id,
        });
        i += 3;
    }
    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_usbip_response() {
        let line1 = "- busid 6-5 (046d:c539)";
        let line2 = "Logitech, Inc. : Lightspeed Receiver (046d:c539)";
        let expected = Some(Device {
            vendor: "Logitech, Inc.".to_string(),
            product: "Lightspeed Receiver".to_string(),
            vendor_id: "046d".to_string(),
            product_id: "c539".to_string(),
            bus_id: "6-5".to_string(),
        });
        assert_eq!(from_lines(line1, line2), expected);
    }

    #[test]
    fn it_parses_usbip_port_response() {
        let input = "\
Imported USB devices
====================
Port 00: <Port in Use> at Full Speed(12Mbps)
       Logitech, Inc. : Lightspeed Receiver (046d:c539)
        1-1 -> usbip://192.168.1.5:3240/6-5
            -> remote bus/dev 006/005
Port 01: <Port in Use> at High Speed(480Mbps)
       unknown vendor : unknown product (1234:5678)
        2-1 -> unknown host, remote port and remote busid
            -> remote bus/dev 001/002
";
        assert_eq!(
            read_connected_devices(input),
            vec![ConnectedDevice {
                device: Device {
                    vendor: "Logitech, Inc.".to_string(),
                    product: "Lightspeed Receiver".to_string(),
                    vendor_id: "046d".to_string(),
                    product_id: "c539".to_string(),
                    bus_id: "6-5".to_string(),
                },
                server_ip: Ipv4Addr::new(192, 168, 1, 5),
                vhci_port: "00".to_string()
            }]
        );
    }

    #[test]
    fn it_parses_empty_usbip_port_response() {
        let input = "\
Imported USB devices
====================
";
        assert_eq!(read_connected_devices(input), vec![]);
    }
}
