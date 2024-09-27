use std::collections::{HashMap, VecDeque};

use tokio::sync::Mutex;

use crate::protocol::{Update, UpdatePayload};

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
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                queue: VecDeque::with_capacity(100),
                update_map: HashMap::with_capacity(100),
                pts: 0,
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

        inner_lock.update_map.insert(new_pts, update);
        inner_lock.queue.push_back(new_pts);
    }

    pub async fn subscribe_or_sync(&self, device_pts: u64) -> Result<(), OutOfSync> {
        let inner_lock = self.inner.lock().await;

        let Some(front_pts) = inner_lock.queue.front().copied() else {
            // TODO: implement subscription.
            // TODO: read from persistent pts.
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
            // TODO: implement subscription.
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
