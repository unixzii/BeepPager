use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Weak;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::protocol::{Update, UpdatePayload};

#[async_trait]
pub trait Subscriber: Send + Sync {
    async fn on_receive_update(&self, update: &Update);
}

struct AnySubscriber(Weak<dyn Subscriber>);

impl Hash for AnySubscriber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = self.0.as_ptr();
        ptr.hash(state);
    }
}

impl PartialEq for AnySubscriber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl Eq for AnySubscriber {}

pub struct OutOfSync {
    pub too_long: bool,
    pub updates: Vec<Update>,
}

pub struct Mailbox {
    inner: Mutex<Inner>,
}

struct Inner {
    // FIXME: performance?
    queue: VecDeque<u64>,
    update_map: HashMap<u64, Update>,
    pts: u64,

    subscribers: HashSet<AnySubscriber>,
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                queue: VecDeque::with_capacity(100),
                update_map: HashMap::with_capacity(100),
                pts: 0,
                subscribers: Default::default(),
            }),
        }
    }

    pub async fn post_update(&self, payload: UpdatePayload) {
        let mut inner_lock = self.inner.lock().await;

        let new_pts = inner_lock.pts + 1;
        inner_lock.pts = new_pts;

        let update = Update {
            pts: new_pts,
            payload,
        };
        debug!("did post update: {update:?}");

        for subscriber in &inner_lock.subscribers {
            if let Some(subscriber) = subscriber.0.upgrade() {
                subscriber.on_receive_update(&update).await;
            }
        }

        inner_lock.update_map.insert(new_pts, update);
        inner_lock.queue.push_back(new_pts);
    }

    pub async fn subscribe_or_sync(
        &self,
        device_pts: u64,
        subscriber: Weak<dyn Subscriber>,
    ) -> Result<(), OutOfSync> {
        let mut inner_lock = self.inner.lock().await;

        let Some(front_pts) = inner_lock.queue.front().copied() else {
            // TODO: read from persistent pts.
            inner_lock.subscribers.insert(AnySubscriber(subscriber));
            return Ok(());
        };

        let too_long = device_pts < front_pts;
        let mut updates = Vec::new();
        for pts in inner_lock.queue.iter() {
            if *pts <= device_pts {
                continue;
            }

            if updates.len() >= 10 {
                break;
            }
            updates.push(
                inner_lock
                    .update_map
                    .get(pts)
                    .expect("internal state is inconsistent")
                    .clone(),
            );
        }

        if updates.is_empty() {
            inner_lock.subscribers.insert(AnySubscriber(subscriber));
            return Ok(());
        }

        Err(OutOfSync { too_long, updates })
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}
