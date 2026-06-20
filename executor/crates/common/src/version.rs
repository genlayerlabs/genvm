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

/// Parses a `u16` starting at `i`, returning the value and the index past the
/// last digit consumed.
const fn parse_u16(s: &[u8], mut i: usize) -> (u16, usize) {
    let mut n: u16 = 0;
    while i < s.len() && s[i] >= b'0' && s[i] <= b'9' {
        n = n * 10 + (s[i] - b'0') as u16;
        i += 1;
    }
    (n, i)
}

const fn const_starts_with(s: &[u8], prefix: &[u8]) -> bool {
    if s.len() < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if s[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn current_version() -> Version {
    if const_starts_with(crate::VERSION.as_bytes(), b"vTEST") {
        // vTEST has no numeric build id, so report this crate's compile-time
        // version (kept at the active dev major/minor via `.genvm-monorepo-root`)
        // with a max patch so it sorts newer than any real release of that line.
        let pkg = env!("CARGO_PKG_VERSION").as_bytes();
        let (major, i) = parse_u16(pkg, 0);
        let (minor, _) = parse_u16(pkg, i + 1); // skip the '.'
        return Version {
            major,
            minor,
            patch: u16::MAX,
        };
    }

    // release build id is `v<major>.<minor>.<patch>-...`
    let id = crate::VERSION.as_bytes();
    let start = if !id.is_empty() && id[0] == b'v' {
        1
    } else {
        0
    };
    let (major, i) = parse_u16(id, start);
    let (minor, i) = parse_u16(id, i + 1); // skip '.'
    let (patch, _) = parse_u16(id, i + 1); // skip '.'
    Version {
        major,
        minor,
        patch,
    }
}

pub const CURRENT: Version = current_version();
