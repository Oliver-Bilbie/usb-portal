# USB Portal

Use a USB device connected to another machine on your network.

The server shares devices, the client uses them. Both services rely on the native linux [USB/IP](https://wiki.archlinux.org/title/USB/IP) implementation, so will not work on MacOS or Windows. The client provides a web UI powered by [htmx](https://htmx.org) which can be accessed locally at http://127.0.0.1:3242 while the client is running.

> [!WARNING]
> The network traffic from the software is not encrypted or password-protected. It would be unwise to use this software anywhere other than on a trusted LAN.

## Requirements

- Linux
- Rust 1.85+
- usbip

## Install

### Dependencies

#### Arch

```sh
sudo pacman -S usbip rust
```

#### Debian/Ubuntu

```sh
sudo apt install usbip build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env
```

*These instructions use [rustup](https://rustup.rs) to install the required Rust version.*

#### Fedora

```sh
sudo dnf install usbip gcc
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env
```

*These instructions use [rustup](https://rustup.rs) to install the required Rust version.*

### Server

On the machine with the USB devices:

```sh
git clone https://github.com/oliver-bilbie/usb-portal.git
cd usb-portal
cargo build --release --bin usb-portal-server
sudo install -Dm755 target/release/usb-portal-server /usr/bin/usb-portal-server
sudo install -Dm644 server/usb-portal-server.service /etc/systemd/system/usb-portal-server.service
sudo systemctl daemon-reload
sudo systemctl enable --now usb-portal-server.service
```

### Client

On the machine that should use them:

```sh
git clone https://github.com/oliver-bilbie/usb-portal.git
cd usb-portal
cargo build --release --bin usb-portal-client
sudo install -Dm755 target/release/usb-portal-client /usr/bin/usb-portal-client
sudo install -Dm644 client/usb-portal-client.service /etc/systemd/system/usb-portal-client.service
sudo systemctl daemon-reload
sudo systemctl enable --now usb-portal-client.service
```

Open http://127.0.0.1:3242 in a browser to access the UI.

## Uninstall

### Server

```sh
sudo systemctl disable --now usb-portal-server.service
sudo rm -f /usr/bin/usb-portal-server /etc/systemd/system/usb-portal-server.service
sudo systemctl daemon-reload
```

### Client

```sh
sudo systemctl disable --now usb-portal-client.service
sudo rm -f /usr/bin/usb-portal-client /etc/systemd/system/usb-portal-client.service
sudo systemctl daemon-reload
```

## Ports

If you run into issues, ensure that the following ports are not blocked by your firewall.

| Port | Where  | What                        |
| ---- | ------ | --------------------------- |
| 3240 | server | USB/IP                      |
| 3241 | server | Control HTTP                |
| 3242 | client | Web UI (local only)         |
| 5353 | both   | Device discovery (optional) |
