//! Bounded frame queue.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};

/// Drops the new item when the queue is full.
pub struct DropChannel<T> {
    tx: SyncSender<T>,
    dropped: AtomicU64,
}

impl<T> DropChannel<T> {
    pub fn pair(depth: usize) -> (Self, Receiver<T>) {
        let (tx, rx) = sync_channel(depth.max(1));
        (
            Self {
                tx,
                dropped: AtomicU64::new(0),
            },
            rx,
        )
    }

    pub fn try_push(&self, item: T) -> bool {
        match self.tx.try_send(item) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    #[allow(dead_code)]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn sender(&self) -> &SyncSender<T> {
        &self.tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_when_full() {
        let (ch, rx) = DropChannel::pair(1);
        assert!(ch.try_push(1));
        assert!(!ch.try_push(2));
        assert_eq!(ch.dropped(), 1);
        assert_eq!(rx.recv().unwrap(), 1);
    }
}
