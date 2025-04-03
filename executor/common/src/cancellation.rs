use std::sync::{atomic::AtomicU32, Arc};

pub struct Token {
    pub chan: tokio::sync::mpsc::Sender<()>,
    pub should_quit: Arc<AtomicU32>,
}

impl Token {
    pub fn is_cancelled(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::SeqCst) != 0
    }
}

pub fn make() -> (Arc<Token>, impl Clone + Fn()) {
    let (sender, receiver) = tokio::sync::mpsc::channel(1);

    let cancel = Arc::new(Token {
        chan: sender,
        should_quit: Arc::new(AtomicU32::new(0)),
    });

    let cancel_copy = cancel.clone();
    let receiver = Arc::new(std::sync::Mutex::new(receiver));

    (cancel, move || {
        cancel_copy
            .should_quit
            .store(1, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut receiver) = receiver.lock() {
            receiver.close();
        }
    })
}
