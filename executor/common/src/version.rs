use anyhow::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "v{}.{}.{}",
            self.major, self.minor, self.patch
        ))
    }
}

impl std::str::FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        let mut parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow::anyhow!("Invalid version format: {}", s));
        }

        parts[0] = parts[0].strip_prefix('v').unwrap_or(parts[0]);

        let major = parts[0]
            .parse::<u16>()
            .with_context(|| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u16>()
            .with_context(|| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse::<u16>()
            .with_context(|| format!("Invalid patch version: {}", parts[2]))?;

        let ret = Version {
            major,
            minor,
            patch,
        };

        if ret == Version::ZERO {
            return Err(anyhow::anyhow!("Version cannot be zero"));
        }

        Ok(ret)
    }
}

impl Version {
    pub const ZERO: Self = Self {
        major: 0,
        minor: 0,
        patch: 0,
    };

    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn is_greater_eq_than(self, other: Version) -> bool {
        if self.major != other.major {
            return self.major > other.major;
        }
        if self.major == 0 {
            (self.minor, self.patch) >= (other.minor, other.patch)
        } else {
            (self.major, self.minor) >= (other.major, other.minor)
        }
    }
}

pub static CURRENT: std::sync::LazyLock<Version> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^v(\d+)\.(\d+)\.(\d+)")
        .unwrap()
        .captures(crate::VERSION)
        .and_then(|caps| {
            Some(Version {
                major: caps[1].parse().ok()?,
                minor: caps[2].parse().ok()?,
                patch: caps[3].parse().ok()?,
            })
        })
        .unwrap()
});
