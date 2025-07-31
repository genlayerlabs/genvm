struct WaiterInner {
    counter: std::sync::atomic::AtomicUsize,
    reached_zero: tokio::sync::Notify,
}

pub struct Waiter(std::sync::Arc<WaiterInner>);

impl Default for Waiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Waiter {
    pub fn new() -> Self {
        Self(std::sync::Arc::new(WaiterInner {
            counter: std::sync::atomic::AtomicUsize::new(1),
            reached_zero: tokio::sync::Notify::new(),
        }))
    }

    pub fn increment(&self) {
        #[allow(unused_variables)]
        let old = self
            .0
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        #[cfg(debug_assertions)]
        if old == 0 {
            panic!("Waiter incremented after reaching zero");
        }
    }

    pub fn decrement(&self) {
        let old_val = self
            .0
            .counter
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        if old_val == 1 {
            self.0.reached_zero.notify_one();
        }
        #[cfg(debug_assertions)]
        if old_val == 0 {
            panic!("Waiter decremented below zero");
        }
    }

    pub async fn wait(&self) {
        if self.0.counter.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            self.0.reached_zero.notified().await;
        }
    }
}
