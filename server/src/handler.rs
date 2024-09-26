use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::extract::{ws, ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;

use crate::protocol::{IncomingMessage, OutgoingMessage};
use crate::state::{AppState, ConnectionHandle};

async fn handle_ws_message(
    msg: ws::Message,
    conn_handle: &Arc<ConnectionHandle>,
    _state: &Arc<AppState>,
) -> Result<()> {
    let conn_id = conn_handle.id();

    debug!("[{conn_id}] received msg: {msg:?}");
    let text_msg = msg.into_text()?;
    let parsed_msg: IncomingMessage = serde_json::from_str(&text_msg)?;

    match parsed_msg {
        IncomingMessage::Login(login_cmd) => {
            conn_handle
                .login(
                    &login_cmd.user_token,
                    &login_cmd.device_token,
                    &login_cmd.secret_key,
                )
                .await;
        }
    };

    Ok(())
}

async fn send_ws_message(msg: OutgoingMessage, ws: &mut ws::WebSocket) -> Result<()> {
    let msg_string = serde_json::to_string(&msg)?;
    ws.send(ws::Message::Text(msg_string)).await?;

    Ok(())
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    info!("got connection from {addr}");
    ws.on_upgrade(|mut ws| async move {
        let state = state;
        let conn_handle = state.register_connection().await;
        let conn_id = conn_handle.id();

        let mut event_chan = conn_handle
            .take_event_channel()
            .await
            .expect("should get the channel");

        loop {
            select! {
                evt = event_chan.recv() => {
                    let Some(evt) = evt else {
                        return;
                    };

                    debug!("[{conn_id}] sending event: {evt:?}");
                    let res = send_ws_message(evt, &mut ws).await;
                    if let Err(err) = res {
                        error!("[{conn_id}] error occurred while sending event: {err}");
                        return;
                    }
                },
                msg = ws.recv() => {
                    let Some(msg_res) = msg else {
                        return;
                    };

                    let msg = match msg_res {
                        Ok(msg) => msg,
                        Err(err) => {
                            error!("[{conn_id}] error occurred while receiving msg: {err}");
                            return;
                        }
                    };

                    let res = handle_ws_message(msg, &conn_handle, &state).await;
                    if let Err(err) = res {
                        error!("[{conn_id}] error occurred while handling msg: {err}");
                        return;
                    }
                },
            }
        }
    })
}
