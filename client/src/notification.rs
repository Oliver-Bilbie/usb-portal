use std::{cmp::Ordering, time};

#[derive(Clone)]
pub struct Notification {
    pub id: uuid::Uuid,
    pub level: log::Level,
    pub timestamp: time::Instant,
    pub message: String,
}

impl Ord for Notification {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .timestamp
            .cmp(&self.timestamp)
            .then(self.level.cmp(&other.level))
    }
}

impl PartialOrd for Notification {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Notification {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Notification {}
