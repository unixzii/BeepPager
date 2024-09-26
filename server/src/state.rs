use std::collections::HashMap;
use std::sync::atomic::{self, AtomicU64};
use std::sync::{Arc, Weak};

use tokio::sync::{mpsc, Mutex, RwLock};

use crate::protocol::OutgoingMessage;

pub struct AppState {
    conns: RwLock<HashMap<u64, Weak<ConnectionHandle>>>,
    id_seed: AtomicU64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            conns: Default::default(),
            id_seed: AtomicU64::new(1),
        }
    }

    pub async fn register_connection(self: &Arc<Self>) -> Arc<ConnectionHandle> {
        let conn_id = self.id_seed.fetch_add(1, atomic::Ordering::Relaxed);
        let conn = Arc::new(ConnectionHandle::new(conn_id, self));
        let mut conns_lock = self.conns.write().await;
        conns_lock.insert(conn_id, Arc::downgrade(&conn));
        conn
    }
}

impl AppState {
    async fn broadcast_event(self: &Arc<Self>, event: OutgoingMessage, sender_conn_id: u64) {
        let conns_lock = self.conns.read().await;
        for (cur_conn_id, conn) in conns_lock.iter() {
            if sender_conn_id == *cur_conn_id {
                continue;
            }
            let Some(conn) = conn.upgrade() else {
                warn!("connection handle dropped without notifying app state");
                continue;
            };
            conn.send_event(event.clone()).await;
        }
    }

    #[inline]
    async fn broadcast_device_online(self: &Arc<Self>, conn_id: u64) {
        self.broadcast_event(OutgoingMessage::DeviceOnline, conn_id)
            .await;
    }

    #[inline]
    async fn broadcast_device_offline(self: &Arc<Self>, conn_id: u64) {
        self.broadcast_event(OutgoingMessage::DeviceOffline, conn_id)
            .await;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConnectionHandle {
    id: u64,
    state: Weak<AppState>,
    chan_tx: mpsc::UnboundedSender<OutgoingMessage>,
    chan_rx: Mutex<Option<mpsc::UnboundedReceiver<OutgoingMessage>>>,
}

impl ConnectionHandle {
    fn new(id: u64, state: &Arc<AppState>) -> Self {
        let (chan_tx, chan_rx) = mpsc::unbounded_channel();
        Self {
            id,
            state: Arc::downgrade(state),
            chan_tx,
            chan_rx: Mutex::new(Some(chan_rx)),
        }
    }

    async fn send_event(&self, event: OutgoingMessage) -> bool {
        self.chan_tx.send(event).is_ok()
    }
}

impl ConnectionHandle {
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    pub async fn take_event_channel(&self) -> Option<mpsc::UnboundedReceiver<OutgoingMessage>> {
        self.chan_rx.lock().await.take()
    }

    pub async fn login(&self, _user_token: &str, _device_token: &str, _secret_key: &str) {
        let Some(state) = self.state.upgrade() else {
            warn!("state has dropped too early");
            return;
        };

        state.broadcast_device_online(self.id).await;
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        let id = self.id();
        info!("[{id}] connection dropped");

        let Some(state) = self.state.upgrade() else {
            warn!("state has dropped too early");
            return;
        };
        let tokio_handle = tokio::runtime::Handle::current();
        tokio_handle.spawn(async move {
            let mut conns_lock = state.conns.write().await;
            conns_lock.remove(&id);
            drop(conns_lock);
            debug!("connection handle ({id}) is removed");

            state.broadcast_device_offline(id).await;
        });
    }
}
