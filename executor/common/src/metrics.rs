use crate::{log_info};

pub struct TimeTracker {
    start: std::time::Instant,
    metric: Metric,
}

static ALL_METRICS: [std::sync::atomic::AtomicU64; Metric::Size as usize] = [
    const { std::sync::atomic::AtomicU64::new(0) }; Metric::Size as usize
];

pub fn log_all() {
    for (i, metric) in ALL_METRICS.iter().enumerate() {
        let value = metric.load(std::sync::atomic::Ordering::Relaxed);
        if value > 0 {
            let micros_dur = std::time::Duration::from_micros(value);
            let metric = unsafe { std::mem::transmute::<_, Metric>(i as u8) };
            log_info!(metric:? = metric, duration:? = micros_dur; "metric");
        }
    }
}

impl TimeTracker {
    pub fn new(metric: Metric) -> Self {
        Self {
            start: std::time::Instant::now(),
            metric,
        }
    }

    fn end_mut(&mut self) -> std::time::Duration {
        let duration = self.start.elapsed();

        ALL_METRICS[self.metric as usize]
            .fetch_add(duration.as_micros() as u64, std::sync::atomic::Ordering::AcqRel);

        duration
    }

    pub fn end(mut self) -> std::time::Duration {
        self.end_mut()
    }
}

pub fn measured<R>(metric: Metric, f: impl FnOnce() -> R) -> R {
    let tracker = TimeTracker::new(metric);
    let result = f();
    tracker.end();

    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Metric {
    ModuleCall,
    HostCall,
    Size
}

pub struct Lock<T>(T, TimeTracker);

impl<T> Lock<T> {
    pub fn new(metric: Metric, value: T) -> Self {
        Self(value, TimeTracker::new(metric))
    }

    pub fn into_inner(self) -> T {
        let mut zelf = std::mem::ManuallyDrop::new(self);
        zelf.1.end_mut();

        unsafe { std::ptr::read(&zelf.0) }
    }
}

impl<T> std::ops::Deref for Lock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Lock<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <T> std::ops::Drop for Lock<T> {
    fn drop(&mut self) {
        self.1.end_mut();
    }
}
