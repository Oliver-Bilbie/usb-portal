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

pub async fn port() -> Result<String, String> {
    run_cmd(&["port"]).await
}

pub async fn list() -> Result<String, String> {
    run_cmd(&["list", "-l"]).await
}

pub async fn bind(bus_id: &str) -> Result<String, String> {
    run_cmd(&["bind", "-b", &bus_id]).await
}

pub async fn unbind(bus_id: &str) -> Result<String, String> {
    run_cmd(&["unbind", "-b", &bus_id]).await
}

pub async fn attach(ip: &Ipv4Addr, bus_id: &str) -> Result<String, String> {
    run_cmd(&["attach", "-r", &ip.to_string(), "-b", bus_id]).await
}

pub async fn detach(vhci_port: &str) -> Result<String, String> {
    run_cmd(&["detach", "-p", vhci_port]).await
}
