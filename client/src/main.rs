mod health_check;
mod helpers;
mod notification;
mod reconnector;
mod render;
mod service;
mod state;
mod tray;

use crate::{
    health_check::HealthCheck, helpers::*, reconnector::Reconnector, state::State, tray::Tray,
};
use axum::{
    Form, Router,
    extract::{
        Query, State as AppState,
        rejection::{FormRejection, QueryRejection},
    },
    http::header,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use log::*;
use serde::Deserialize;
use std::{net::Ipv4Addr, sync::Arc};
use tokio::net::TcpListener;
use usb_portal_lib::prelude::*;

async fn handle_connected(AppState(state): AppState<Arc<State>>) -> Html<String> {
    Html(render::connected(&state))
}

async fn handle_available(AppState(state): AppState<Arc<State>>) -> Html<String> {
    state.wait_for_discovery().await;
    Html(render::available(&state).await)
}

async fn handle_refresh(AppState(state): AppState<Arc<State>>) -> Html<String> {
    discover_servers(&state).await;
    Html(render::body(&state).await)
}

#[derive(Deserialize)]
struct DismissNotificationForm {
    id: String,
}
async fn handle_dismiss_notification(
    AppState(state): AppState<Arc<State>>,
    Form(form): Form<DismissNotificationForm>,
) -> Html<String> {
    state.dismiss_notification(&form.id);
    Html(render::notifications(&state))
}

#[derive(Deserialize)]
struct AddServerForm {
    ip: Ipv4Addr,
    port: u16,
}
async fn handle_add_server(
    AppState(state): AppState<Arc<State>>,
    form_result: Result<Query<AddServerForm>, QueryRejection>,
) -> Html<String> {
    match form_result {
        Ok(form) => {
            add_manual_server(form.ip, form.port, &state).await;
        }
        Err(e) => {
            state.push_notification(Level::Error, e.body_text());
        }
    };
    Html(render::body(&state).await)
}

#[derive(Deserialize)]
struct AttachForm {
    ip: Ipv4Addr,
    bus_id: String,
}
async fn handle_attach(
    AppState(state): AppState<Arc<State>>,
    form_result: Result<Form<AttachForm>, FormRejection>,
) -> Html<String> {
    match form_result {
        Ok(form) => {
            let http_port = server_http_port(&state, &form.ip);
            attach_device(&form.ip, http_port, &form.bus_id, false, &state).await;
        }
        Err(e) => {
            state.push_notification(Level::Error, e.body_text());
        }
    };
    Html(render::body(&state).await)
}

#[derive(Deserialize)]
struct DetachForm {
    ip: Ipv4Addr,
    bus_id: String,
    vhci_port: String,
}
async fn handle_detach(
    AppState(state): AppState<Arc<State>>,
    form_result: Result<Form<DetachForm>, FormRejection>,
) -> Html<String> {
    match form_result {
        Ok(form) => {
            detach_device(&form.ip, &form.bus_id, &form.vhci_port, &state).await;
        }
        Err(e) => {
            state.push_notification(Level::Error, e.body_text());
        }
    };
    Html(render::body(&state).await)
}

async fn handle_index(AppState(state): AppState<Arc<State>>) -> Html<String> {
    let s = state.clone();
    state.push_discovery(tokio::spawn(async move { discover_servers(&s).await }));
    const INDEX: &str = include_str!("../assets/index.html");
    Html(INDEX.to_string())
}

async fn handle_styles() -> impl IntoResponse {
    const STYLES: &str = include_str!("../assets/styles.css");
    ([(header::CONTENT_TYPE, "text/css")], STYLES)
}

async fn handle_htmx_js() -> impl IntoResponse {
    const HTMX_JS: &str = include_str!("../assets/htmx.min.js");
    ([(header::CONTENT_TYPE, "text/javascript")], HTMX_JS)
}

async fn handle_icon() -> impl IntoResponse {
    const ICON: &[u8] = include_bytes!("../assets/favicon.ico");
    ([(header::CONTENT_TYPE, "image/x-icon")], ICON)
}

async fn handle_geist_sans() -> impl IntoResponse {
    const FONT: &[u8] = include_bytes!("../assets/Geist-Variable.woff2");
    ([(header::CONTENT_TYPE, "font/woff2")], FONT)
}

async fn handle_geist_mono() -> impl IntoResponse {
    const FONT: &[u8] = include_bytes!("../assets/GeistMono-Variable.woff2");
    ([(header::CONTENT_TYPE, "font/woff2")], FONT)
}

#[tokio::main]
async fn main() {
    logger::init();
    let state: Arc<State> = Arc::new(State::new());
    let mut tray = Tray::init();
    let mut hc = HealthCheck::init(state.clone());
    let mut rc = Reconnector::init(state.clone());

    let app = Router::new()
        .route("/", get(handle_index))
        .route("/refresh", get(handle_refresh))
        .route("/available", get(handle_available))
        .route("/connected", get(handle_connected))
        .route("/add_manual_server", get(handle_add_server))
        .route("/device/attach", post(handle_attach))
        .route("/device/detach", post(handle_detach))
        .route("/notification/dismiss", post(handle_dismiss_notification))
        .route("/styles.css", get(handle_styles))
        .route("/htmx.min.js", get(handle_htmx_js))
        .route("/favicon.ico", get(handle_icon))
        .route("/fonts/Geist-Variable.woff2", get(handle_geist_sans))
        .route("/fonts/GeistMono-Variable.woff2", get(handle_geist_mono))
        .with_state(state.clone());

    let listener = TcpListener::bind(format!("127.0.0.1:{}", DEFAULT_CLIENT_PORT))
        .await
        .unwrap();
    info!(
        "Starting client on http://127.0.0.1:{}",
        DEFAULT_CLIENT_PORT
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    println!();
    hc.shutdown().await;
    rc.shutdown().await;
    tray.shutdown().await;
    release_all(state).await;
    info!("The client has stopped");
}
