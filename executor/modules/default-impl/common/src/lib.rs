pub fn run_with_termination<F>(f: F, timeout_handle: *mut u32) -> Option<F::Output>
where
    F: core::future::Future + Send,
    F::Output: Send,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let should_quit = unsafe { std::sync::atomic::AtomicU32::from_ptr(timeout_handle) };

    let selector = async {
        let tracker_fut = async {
            while should_quit.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                tokio::time::sleep(tokio::time::Duration::new(0, 1_000_000)).await;
            }
        };
        let waiter = tokio::spawn(tracker_fut);

        loop {
            tokio::select! {
                val = f => return Some(val),
                _ = waiter => {
                    return None
                },
            }
        }
    };

    let a = rt.block_on(selector);
    rt.shutdown_background();
    a
}
