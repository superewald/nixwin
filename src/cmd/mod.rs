mod config;
mod install;
mod manage;
mod setup;

pub use config::*;
pub use install::*;
pub use manage::*;
pub use setup::*;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    #[value(name = "x86")]
    X86,
    #[value(name = "x86_64")]
    X86_64,
    #[value(name = "aarch")]
    Aarch,
    #[value(name = "aarch64")]
    Aarch64,
}

impl Arch {
    pub const fn as_str(self) -> &'static str {
        match self {
            Arch::X86 => "x86",
            Arch::X86_64 => "x86_64",
            Arch::Aarch => "aarch",
            Arch::Aarch64 => "aarch64",
        }
    }

    /// The MS (manifest) notation for the architecture, eg `x64`
    pub const fn as_ms_str(self) -> &'static str {
        match self {
            Arch::X86 => "x86",
            Arch::X86_64 => "x64",
            Arch::Aarch => "arm",
            Arch::Aarch64 => "arm64",
        }
    }

    /// The MSVC-style target triple used by clang
    pub const fn as_triple(self) -> &'static str {
        match self {
            Arch::X86 => "i686-pc-windows-msvc",
            Arch::X86_64 => "x86_64-pc-windows-msvc",
            Arch::Aarch => "armv7-pc-windows-msvc",
            Arch::Aarch64 => "aarch64-pc-windows-msvc",
        }
    }

    /// Architecture of the host system, if supported
    pub fn host() -> Option<Self> {
        match std::env::consts::ARCH {
            "x86" => Some(Self::X86),
            "x86_64" => Some(Self::X86_64),
            "arm" => Some(Self::Aarch),
            "aarch64" => Some(Self::Aarch64),
            _ => None,
        }
    }

    /// Canonical default-selection order (x86_64 first)
    pub const fn rank(self) -> u8 {
        match self {
            Arch::X86_64 => 0,
            Arch::X86 => 1,
            Arch::Aarch64 => 2,
            Arch::Aarch => 3,
        }
    }

    pub fn to_xwin(self) -> xwin::Arch {
        match self {
            Arch::X86 => xwin::Arch::X86,
            Arch::X86_64 => xwin::Arch::X86_64,
            Arch::Aarch => xwin::Arch::Aarch,
            Arch::Aarch64 => xwin::Arch::Aarch64,
        }
    }

    /// Sum of the xwin arch bitflags, as expected by the xwin api
    pub fn bits(archs: &[Arch]) -> u32 {
        archs.iter().map(|a| a.to_xwin() as u32).sum()
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Arch {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "x86" => Arch::X86,
            "x86_64" => Arch::X86_64,
            "aarch" => Arch::Aarch,
            "aarch64" => Arch::Aarch64,
            _ => anyhow::bail!(
                "unknown architecture '{s}', expected one of: x86, x86_64, aarch, aarch64"
            ),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    #[value(name = "desktop")]
    Desktop,
    #[value(name = "onecore")]
    OneCore,
    #[value(name = "store")]
    Store,
    #[value(name = "spectre")]
    Spectre,
}

impl Variant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Variant::Desktop => "desktop",
            Variant::OneCore => "onecore",
            Variant::Store => "store",
            Variant::Spectre => "spectre",
        }
    }

    pub fn to_xwin(self) -> xwin::Variant {
        match self {
            Variant::Desktop => xwin::Variant::Desktop,
            Variant::OneCore => xwin::Variant::OneCore,
            Variant::Store => xwin::Variant::Store,
            Variant::Spectre => xwin::Variant::Spectre,
        }
    }

    pub fn bits(variants: &[Variant]) -> u32 {
        variants.iter().map(|v| v.to_xwin() as u32).sum()
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Variant {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "desktop" => Variant::Desktop,
            "onecore" => Variant::OneCore,
            "store" => Variant::Store,
            "spectre" => Variant::Spectre,
            _ => anyhow::bail!(
                "unknown variant '{s}', expected one of: desktop, onecore, store, spectre"
            ),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Feature {
    #[value(name = "debug")]
    Debug,
    #[value(name = "atl")]
    Atl,
}

impl Feature {
    pub const fn as_str(self) -> &'static str {
        match self {
            Feature::Debug => "debug",
            Feature::Atl => "atl",
        }
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Feature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "debug" => Feature::Debug,
            "atl" => Feature::Atl,
            _ => anyhow::bail!("unknown feature '{s}', expected one of: debug, atl"),
        })
    }
}

/// Renders a list as `[a,b]`, matching the README inspect format
pub fn fmt_list<T: fmt::Display>(items: &[T]) -> String {
    let joined: Vec<String> = items.iter().map(|i| i.to_string()).collect();
    format!("[{}]", joined.join(","))
}

/// Parses a comma-separated list, eg from `--archs x86,x86_64` or
/// `nixwin config default.arches x86,x86_64`
pub fn parse_csv_list<T>(value: &str) -> anyhow::Result<Vec<T>>
where
    T: std::str::FromStr,
    T::Err: Into<anyhow::Error>,
{
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<T>().map_err(Into::into))
        .collect()
}
