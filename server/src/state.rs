use std::collections::hash_map;
use std::collections::HashMap;
use std::sync::atomic::{self, AtomicU64};
use std::sync::{Arc, Weak};

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::mailbox::{Mailbox, Subscriber};
use crate::protocol::{
    LoginCommand, NewMessageUpdate, OutgoingMessage, SendMessageCommand, SyncUpdates, Update,
    UpdatePayload,
};

struct UserState {
    mailbox: Arc<Mailbox>,
}

pub struct AppState {
    conns: RwLock<HashMap<u64, Weak<ConnectionHandle>>>,
    users: RwLock<HashMap<String, UserState>>,
    id_seed: AtomicU64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            conns: Default::default(),
            users: Default::default(),
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

    async fn login(self: &Arc<Self>, conn_handle: &ConnectionHandle, cmd: &LoginCommand) {
        let mut users_lock = self.users.write().await;
        let mailbox = match users_lock.entry(cmd.user_token.to_owned()) {
            hash_map::Entry::Occupied(entry) => {
                let user_state = entry.get();
                Arc::clone(&user_state.mailbox)
            }
            hash_map::Entry::Vacant(vacant) => {
                info!("initializing user: {}", cmd.user_token);

                let mailbox = Default::default();
                vacant.insert(UserState {
                    mailbox: Arc::clone(&mailbox),
                });

                mailbox
            }
        };
        drop(users_lock);

        *conn_handle.mailbox.write().await = Some(mailbox);
        *conn_handle.user_token.write().await = Some(cmd.user_token.to_owned());

        self.broadcast_device_online(conn_handle.id).await;
        conn_handle.send_event(OutgoingMessage::LoggedIn).await;
    }

    async fn send_message(
        self: &Arc<Self>,
        conn_handle: &ConnectionHandle,
        cmd: &SendMessageCommand,
    ) {
        let users_lock = self.users.read().await;
        let Some(user_state) = users_lock.get(&cmd.receiver) else {
            error!("receiver ({}) is not found", cmd.receiver);
            return;
        };
        let mailbox = Arc::clone(&user_state.mailbox);
        drop(users_lock);

        let user_token_lock = conn_handle.user_token.read().await;
        let Some(user_token) = user_token_lock.clone() else {
            error!("[{}] send message before logging in", conn_handle.id);
            return;
        };
        drop(user_token_lock);

        mailbox
            .post_update(UpdatePayload::NewMessage(NewMessageUpdate {
                sender: user_token,
                contents: cmd.contents.to_owned(),
            }))
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
    mailbox: RwLock<Option<Arc<Mailbox>>>,
    user_token: RwLock<Option<String>>,
}

impl ConnectionHandle {
    fn new(id: u64, state: &Arc<AppState>) -> Self {
        let (chan_tx, chan_rx) = mpsc::unbounded_channel();
        Self {
            id,
            state: Arc::downgrade(state),
            chan_tx,
            chan_rx: Mutex::new(Some(chan_rx)),
            mailbox: Default::default(),
            user_token: Default::default(),
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

    pub async fn login(&self, cmd: &LoginCommand) {
        let id = self.id();
        info!(
            "[{id}] request to login (user_token: {}, device_token: {})",
            cmd.user_token, cmd.device_token
        );

        let Some(state) = self.state.upgrade() else {
            warn!("state has dropped too early");
            return;
        };

        state.login(self, cmd).await;
    }

    pub async fn subscribe_or_sync_updates(self: &Arc<Self>, device_pts: u64) {
        let id = self.id();
        info!("[{id}] request to subscribe or sync updates (device_pts: {device_pts})");

        let mailbox_lock = self.mailbox.read().await;
        let Some(mailbox) = mailbox_lock.clone() else {
            error!("[{id}] subscribe updates before logging in");
            return;
        };
        drop(mailbox_lock);

        let subscriber = Arc::<Self>::downgrade(self);
        match mailbox.subscribe_or_sync(device_pts, subscriber).await {
            Ok(_) => {
                info!("[{id}] update subscription started");
                self.send_event(OutgoingMessage::SyncUpdates(SyncUpdates {
                    too_long: false,
                    synced: true,
                    updates: vec![],
                }))
                .await;
            }
            Err(out_of_sync) => {
                info!(
                    "[{id}] syncing {} updates (too_long: {})",
                    out_of_sync.updates.len(),
                    out_of_sync.too_long
                );
                self.send_event(OutgoingMessage::SyncUpdates(SyncUpdates {
                    too_long: out_of_sync.too_long,
                    synced: false,
                    updates: out_of_sync.updates,
                }))
                .await;
            }
        }
    }

    pub async fn send_message(&self, cmd: &SendMessageCommand) {
        let id = self.id();
        info!(
            "[{id}] request to send message (receiver: {})",
            cmd.receiver
        );

        let Some(state) = self.state.upgrade() else {
            warn!("state has dropped too early");
            return;
        };

        state.send_message(self, cmd).await;
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

#[async_trait]
impl Subscriber for ConnectionHandle {
    async fn on_receive_update(&self, update: &Update) {
        let update = update.clone();
        self.send_event(OutgoingMessage::Update(update)).await;
    }
}
