use ksni::{MenuItem, TrayMethods, menu::*};
use log::*;

use crate::service::Service;

pub struct Tray {
    service: Service,
}

impl Tray {
    pub fn init() -> Tray {
        let loop_fn = {
            async || {
                let t = TrayData {};
                match t.spawn().await {
                    Ok(_) => std::future::pending().await,
                    Err(e) => {
                        error!("Unable to start tray\n{}", e);
                        Err(())
                    }
                }
            }
        };
        let service = Service::init("Tray", loop_fn);
        Tray { service }
    }

    pub async fn shutdown(&mut self) {
        self.service.shutdown().await;
    }
}

struct TrayData {}

impl ksni::Tray for TrayData {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        const ICON_BYTES: &[u8] = include_bytes!("../assets/favicon.ico");
        vec![ksni::Icon {
            width: 256,
            height: 256,
            data: ICON_BYTES.to_vec(),
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open Web UI".to_string(),
                activate: Box::new(|_: &mut Self| println!(":-)")),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Exit".to_string(),
                activate: Box::new(|_: &mut Self| println!(":-(")),
                ..Default::default()
            }
            .into(),
        ]
    }
}
