pub mod metric {
    #[derive(serde::Serialize)]
    pub struct Time(std::sync::atomic::AtomicU64);
    #[derive(serde::Serialize)]
    pub struct Count(std::sync::atomic::AtomicU64);

    impl std::fmt::Debug for Time {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let i = self.0.load(std::sync::atomic::Ordering::SeqCst);
            let i = std::time::Duration::from_micros(i);
            write!(f, "{i:?}")
        }
    }

    impl std::fmt::Debug for Count {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let i = self.0.load(std::sync::atomic::Ordering::SeqCst);
            write!(f, "{i}")
        }
    }

    impl Time {
        pub fn new() -> Self {
            Self(std::sync::atomic::AtomicU64::new(0))
        }

        pub fn add(&self, value: std::time::Duration) {
            self.0.fetch_add(
                value.as_micros() as u64,
                std::sync::atomic::Ordering::AcqRel,
            );
        }
    }

    impl Default for Time {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Count {
        pub fn new() -> Self {
            Self(std::sync::atomic::AtomicU64::new(0))
        }

        pub fn increment(&self) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        }
    }

    impl Default for Count {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod tracker {
    use crate::sync::DArc;

    use super::metric;

    pub struct Time(std::time::Instant, DArc<metric::Time>);

    impl Time {
        pub fn new(metric: DArc<metric::Time>) -> Self {
            Self(std::time::Instant::now(), metric)
        }
    }

    impl std::ops::Drop for Time {
        fn drop(&mut self) {
            let duration = self.0.elapsed();
            self.1.add(duration);
        }
    }
}
