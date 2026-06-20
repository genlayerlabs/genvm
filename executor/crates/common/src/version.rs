use anyhow::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

        let patch_str = parts[2].split('-').next().unwrap_or(parts[2]);

        let major = parts[0]
            .parse::<u16>()
            .with_context(|| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse::<u16>()
            .with_context(|| format!("Invalid minor version: {}", parts[1]))?;
        let patch = patch_str
            .parse::<u16>()
            .with_context(|| format!("Invalid patch version: {}", patch_str))?;

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
}

pub static CURRENT: std::sync::LazyLock<Version> = std::sync::LazyLock::new(|| {
    if crate::VERSION.starts_with("vTEST") {
        // vTEST has no numeric build id, so report this crate's compile-time
        // version (kept at the active dev major/minor via `.genvm-monorepo-root`)
        // with a max patch so it sorts newer than any real release of that line.
        let mut parts = env!("CARGO_PKG_VERSION").split('.');
        let major = parts
            .next()
            .and_then(|x| x.parse().ok())
            .expect("CARGO_PKG_VERSION must have a numeric major");
        let minor = parts
            .next()
            .and_then(|x| x.parse().ok())
            .expect("CARGO_PKG_VERSION must have a numeric minor");
        return Version {
            major,
            minor,
            patch: u16::MAX,
        };
    }

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
