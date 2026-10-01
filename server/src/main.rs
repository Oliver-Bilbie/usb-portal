use axum::{Router, extract::Path, http::StatusCode, routing::get};
use gethostname::gethostname;
use local_ip_address::linux::local_ip;
use log::*;
use tokio::net::TcpListener;
use usb_portal_lib::prelude::*;

async fn handle_list() -> Result<String, (StatusCode, String)> {
    let task = UsbipTask::List;
    let resp = match task.run().await {
        Ok(v) => v,
        Err(msg) => return Err((StatusCode::INTERNAL_SERVER_ERROR, msg)),
    };
    let devices = read_devices(&resp);
    match serde_json::to_string(&devices) {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to serialize device list".to_string(),
        )),
    }
}

async fn handle_bind(Path(id): Path<String>) -> Result<String, (StatusCode, String)> {
    let task = UsbipTask::Bind(BindArgs { bus_id: &id });
    match task.run().await {
        Ok(v) => Ok(v),
        Err(msg) => return Err((StatusCode::INTERNAL_SERVER_ERROR, msg)),
    }
}

async fn handle_unbind(Path(id): Path<String>) -> Result<String, (StatusCode, String)> {
    let task = UsbipTask::Unbind(UnbindArgs { bus_id: &id });
    match task.run().await {
        Ok(v) => Ok(v),
        Err(msg) => return Err((StatusCode::INTERNAL_SERVER_ERROR, msg)),
    }
}

async fn handle_hostname() -> String {
    gethostname().to_string_lossy().to_string()
}

async fn handle_ping() -> String {
    "pong".to_string()
}

async fn release_all() {
    let task = UsbipTask::List;
    let devices = match task.run().await {
        Ok(v) => read_devices(&v),
        Err(msg) => {
            error!("Unable to release devices\n{}", msg);
            return;
        }
    };
    for d in devices {
        match handle_unbind(Path(d.bus_id.clone())).await {
            Ok(_) => {
                info!("Released device {}", &d);
            }
            Err((_, msg)) => {
                if !msg.contains("Device is not bound to usbip-host driver") {
                    error!("Unable to release device {}\n{}", d, msg);
                }
            }
        }
    }
    info!("the server has stopped");
}

#[tokio::main]
async fn main() {
    logger::init();

    let ip = local_ip().expect("This machine does not appear to be connected to a network");
    info!("Starting server...");
    info!("IP = {}", ip);
    info!("Port = {}", DEFAULT_SERVER_PORT);

    let app = Router::new()
        .route("/list", get(handle_list))
        .route("/bind/{id}", get(handle_bind))
        .route("/unbind/{id}", get(handle_unbind))
        .route("/hostname", get(handle_hostname))
        .route("/ping", get(handle_ping));

    let advertisement = match advertise(DEFAULT_SERVER_PORT) {
        Ok(ad) => Some(ad),
        Err(msg) => {
            error!("unable to advertise on the network\n{}", msg);
            None
        }
    };

    let listener = TcpListener::bind(format!("0.0.0.0:{}", DEFAULT_SERVER_PORT))
        .await
        .unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    drop(advertisement);
    release_all().await;
}
