use std::net::Ipv4Addr;
use tokio::process::Command;

async fn run_cmd(args: &[&str]) -> Result<String, String> {
    let output = Command::new("usbip")
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

async fn port() -> Result<String, String> {
    run_cmd(&["port"]).await
}

async fn list() -> Result<String, String> {
    run_cmd(&["list", "-l"]).await
}

async fn bind(bus_id: &str) -> Result<String, String> {
    run_cmd(&["bind", "-b", &bus_id]).await
}

async fn unbind(bus_id: &str) -> Result<String, String> {
    run_cmd(&["unbind", "-b", &bus_id]).await
}

async fn attach(ip: &Ipv4Addr, bus_id: &str) -> Result<String, String> {
    run_cmd(&["attach", "-r", &ip.to_string(), "-b", bus_id]).await
}

async fn detach(vhci_port: &str) -> Result<String, String> {
    run_cmd(&["detach", "-p", vhci_port]).await
}

pub enum UsbipTask<'a> {
    Port,
    List,
    Bind(BindArgs<'a>),
    Unbind(UnbindArgs<'a>),
    Attach(AttachArgs<'a>),
    Detach(DetachArgs<'a>),
}

pub struct BindArgs<'a> {
    pub bus_id: &'a str,
}

pub struct UnbindArgs<'a> {
    pub bus_id: &'a str,
}

pub struct AttachArgs<'a> {
    pub ip: &'a Ipv4Addr,
    pub bus_id: &'a str,
}

pub struct DetachArgs<'a> {
    pub vhci_port: &'a str,
}

impl<'a> UsbipTask<'a> {
    pub async fn run(&self) -> Result<String, String> {
        match self {
            Self::Port => port().await,
            Self::List => list().await,
            Self::Bind(args) => bind(args.bus_id).await,
            Self::Unbind(args) => unbind(args.bus_id).await,
            Self::Attach(args) => attach(args.ip, args.bus_id).await,
            Self::Detach(args) => detach(args.vhci_port).await,
        }
    }
}
