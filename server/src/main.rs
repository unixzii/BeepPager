mod handler;
mod mailbox;
mod protocol;
mod state;

#[macro_use]
extern crate log;

#[macro_use]
extern crate tokio;

use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use anyhow::Result;
use axum::routing::get;
use axum::Router;

use crate::state::AppState;

fn init_logger() {
    #[cfg(debug_assertions)]
    {
        let mut builder = pretty_env_logger::formatted_timed_builder();
        builder.parse_default_env();
        builder.filter_level(log::LevelFilter::Debug);
        builder.init();
    }
    #[cfg(not(debug_assertions))]
    pretty_env_logger::init_timed();
}

fn get_env_or<K: AsRef<OsStr>>(key: K, default: &'static str) -> Cow<'static, str> {
    if let Ok(value) = env::var(key) {
        return Cow::Owned(value);
    }
    Cow::Borrowed(default)
}

async fn serve(port: u16) -> Result<()> {
    let state = Arc::new(AppState::default());

    let app = Router::new()
        .route("/ws", get(handler::ws_handler))
        .with_state(state);

    let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    let listener = axum_server::bind(listen_addr);

    info!("server started at port {port}!");
    listener
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    init_logger();

    let port_str = get_env_or("BP_PORT", "5020");
    let port: u16 = match port_str.parse() {
        Ok(value) => value,
        Err(err) => {
            warn!("invalid port value \"{port_str}\": {err}");
            5000
        }
    };

    if let Err(err) = serve(port).await {
        error!("error occurred: {err}");
    }
}
