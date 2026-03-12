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

    #[derive(Debug, Default, serde::Serialize)]
    pub struct TokenMetrics {
        pub input: u32,
        pub output: u32,
        pub total: u32,
    }

    impl TokenMetrics {
        pub fn add(&mut self, input: Option<u32>, output: Option<u32>, total: Option<u32>) {
            if let Some(v) = input {
                self.input += v;
            }
            if let Some(v) = output {
                self.output += v;
            }
            if let Some(v) = total {
                self.total += v;
            }
        }
    }

    pub struct TokenMetricsMap(std::sync::Mutex<std::collections::BTreeMap<String, TokenMetrics>>);

    impl TokenMetricsMap {
        pub fn new() -> Self {
            Self(std::sync::Mutex::new(std::collections::BTreeMap::new()))
        }

        pub fn record(
            &self,
            key: String,
            input: Option<u32>,
            output: Option<u32>,
            total: Option<u32>,
        ) {
            let mut map = self.0.lock().unwrap();
            map.entry(key).or_default().add(input, output, total);
        }
    }

    impl Default for TokenMetricsMap {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Debug for TokenMetricsMap {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let map = self.0.lock().unwrap();
            f.debug_tuple("TokenMetricsMap").field(&*map).finish()
        }
    }

    impl serde::Serialize for TokenMetricsMap {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let map = self.0.lock().unwrap();
            map.serialize(serializer)
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
