use crate::cmd::{Arch, Feature, Variant};
use crate::Ctx;
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn default_channel() -> String {
    "release".to_owned()
}

/// The full configuration of an installed sysroot, stored as
/// `<tag>/nixwin.json` and used for lockfiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SysrootConfig {
    pub tag: String,
    pub manifest: u8,
    /// VS manifest channel used to resolve the manifest
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub archs: Vec<Arch>,
    #[serde(default)]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub features: Vec<Feature>,
    pub sdk: String,
    pub crt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcr: Option<String>,
}

/// Lenient version of [`SysrootConfig`] used while resolving configuration
/// from flags, lockfiles and existing sysroots.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    pub tag: Option<String>,
    pub manifest: Option<u8>,
    pub channel: Option<String>,
    pub archs: Vec<Arch>,
    pub variants: Vec<Variant>,
    pub features: Vec<Feature>,
    pub sdk: Option<String>,
    pub crt: Option<String>,
}

impl From<SysrootConfig> for InputConfig {
    fn from(cfg: SysrootConfig) -> Self {
        Self {
            tag: Some(cfg.tag),
            manifest: Some(cfg.manifest),
            channel: Some(cfg.channel),
            archs: cfg.archs,
            variants: cfg.variants,
            features: cfg.features,
            sdk: Some(cfg.sdk),
            crt: Some(cfg.crt),
        }
    }
}

/// Machine level default values (`default.*` config keys)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub archs: Vec<Arch>,
    #[serde(default)]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub features: Vec<Feature>,
    #[serde(default)]
    pub manifest: Option<u8>,
    #[serde(default)]
    pub channel: Option<String>,
}

/// Machine level configuration, stored as `$NIXWIN_DATA/config.json`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineConfig {
    #[serde(default)]
    pub default: Defaults,
    /// Template overrides: `tpl.<name>` -> template source path
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub tpl: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub cmake: bool,
    #[serde(default)]
    pub wine: bool,
}

pub fn validate_tag(tag: &str) -> Result<()> {
    if tag.is_empty() || tag == "." || tag == ".." {
        bail!("invalid sysroot tag '{tag}'");
    }
    if tag
        .chars()
        .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control())
    {
        bail!("sysroot tag '{tag}' contains invalid path characters");
    }
    Ok(())
}

/// CLI overrides for [`resolve_config`]
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub tag: Option<String>,
    /// `MANIFEST_VERSION` positional argument (tag fallback)
    pub positional_tag: Option<String>,
    pub manifest: Option<u8>,
    pub channel: Option<String>,
    pub archs: Option<Vec<Arch>>,
    pub variants: Option<Vec<Variant>>,
    pub features: Option<Vec<Feature>>,
    pub sdk: Option<String>,
    pub crt: Option<String>,
}

/// The resolved install configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub tag: String,
    pub manifest: u8,
    pub channel: String,
    pub archs: Vec<Arch>,
    pub variants: Vec<Variant>,
    pub features: Vec<Feature>,
    pub sdk: Option<String>,
    pub crt: Option<String>,
}

/// Resolves the effective install configuration.
///
/// Scalars (`tag`, `manifest`, `channel`, `sdk`, `crt`): flags > lockfile >
/// existing tag config > machine defaults > builtin defaults.
/// Lists (`archs`, `variants`, `features`) are *unions* of tag config,
/// lockfile and flags; machine defaults only fill in when that union is
/// empty (fill-only fallback).
pub fn resolve_config(
    tag_cfg: Option<InputConfig>,
    file_cfg: Option<InputConfig>,
    ov: &Overrides,
    defaults: &Defaults,
    host_arch: Option<Arch>,
    ci: bool,
) -> Result<Resolved> {
    let tag = ov
        .tag
        .clone()
        .or_else(|| file_cfg.as_ref().and_then(|c| c.tag.clone()))
        .or_else(|| tag_cfg.as_ref().and_then(|c| c.tag.clone()))
        .or_else(|| ov.positional_tag.clone())
        .or_else(|| defaults.tag.clone())
        .context("missing sysroot tag, use --tag or the MANIFEST_VERSION argument")?;
    validate_tag(&tag)?;

    let manifest = ov
        .manifest
        .or(file_cfg.as_ref().and_then(|c| c.manifest))
        .or(tag_cfg.as_ref().and_then(|c| c.manifest))
        .or(defaults.manifest)
        .unwrap_or(17);

    let channel = ov
        .channel
        .clone()
        .or_else(|| file_cfg.as_ref().and_then(|c| c.channel.clone()))
        .or_else(|| tag_cfg.as_ref().and_then(|c| c.channel.clone()))
        .or_else(|| defaults.channel.clone())
        .unwrap_or_else(default_channel);

    let archs = union3(
        tag_cfg.as_ref().map(|c| &c.archs),
        file_cfg.as_ref().map(|c| &c.archs),
        ov.archs.as_ref(),
    );
    let archs = if archs.is_empty() {
        if defaults.archs.is_empty() {
            vec![host_arch.context(
                "unable to determine host architecture, specify it with --archs",
            )?]
        } else {
            defaults.archs.clone()
        }
    } else {
        archs
    };

    let variants = union3(
        tag_cfg.as_ref().map(|c| &c.variants),
        file_cfg.as_ref().map(|c| &c.variants),
        ov.variants.as_ref(),
    );
    let variants = if variants.is_empty() && defaults.variants.is_empty() {
        vec![Variant::Desktop]
    } else if variants.is_empty() {
        defaults.variants.clone()
    } else {
        variants
    };

    let mut features = union3(
        tag_cfg.as_ref().map(|c| &c.features),
        file_cfg.as_ref().map(|c| &c.features),
        ov.features.as_ref(),
    );
    if features.is_empty() && !defaults.features.is_empty() {
        features = defaults.features.clone();
    } else if features.is_empty() && !ci {
        features.push(Feature::Debug);
    }

    Ok(Resolved {
        tag,
        manifest,
        channel,
        archs,
        variants,
        features,
        sdk: ov
            .sdk
            .clone()
            .or_else(|| file_cfg.as_ref().and_then(|c| c.sdk.clone()))
            .or_else(|| tag_cfg.as_ref().and_then(|c| c.sdk.clone())),
        crt: ov
            .crt
            .clone()
            .or_else(|| file_cfg.as_ref().and_then(|c| c.crt.clone()))
            .or_else(|| tag_cfg.as_ref().and_then(|c| c.crt.clone())),
    })
}

fn union3<T: Copy + PartialEq>(
    a: Option<&Vec<T>>,
    b: Option<&Vec<T>>,
    c: Option<&Vec<T>>,
) -> Vec<T> {
    let mut out = Vec::new();
    for list in [a, b, c].into_iter().flatten() {
        for item in list {
            if !out.contains(item) {
                out.push(*item);
            }
        }
    }
    out
}

pub fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read(path)
        .with_context(|| format!("unable to read {}", path.display()))?;
    serde_json::from_slice(&contents)
        .map(Some)
        .with_context(|| format!("unable to parse {}", path.display()))
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("unable to create {}", parent.display()))?;
    }
    let file = std::fs::File::create(path)
        .with_context(|| format!("unable to create {}", path.display()))?;
    serde_json::to_writer_pretty(std::io::BufWriter::new(file), value)
        .with_context(|| format!("unable to write {}", path.display()))?;
    Ok(())
}

/// Loads a nixwin.json configuration/lockfile leniently (all fields optional)
pub fn load_input_config(path: &Path) -> Result<Option<InputConfig>> {
    load_json(path)
}

/// Loads the machine level configuration (`$NIXWIN_DATA/config.json`)
pub fn load_machine_config(ctx: &Ctx) -> Result<MachineConfig> {
    Ok(load_json(&ctx.paths.machine_config())?.unwrap_or_default())
}

/// Points `$NIXWIN_SYSROOT` at the given tag and records it as
/// `default.tag` in the machine configuration
pub fn set_default_tag(ctx: &Ctx, tag: &str) -> Result<()> {
    ctx.paths.set_default(tag)?;
    let path = ctx.paths.machine_config();
    let mut machine: MachineConfig = load_json(&path)?.unwrap_or_default();
    machine.default.tag = Some(tag.to_owned());
    save_json(&path, &machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(tag: &str, archs: Vec<Arch>, features: Vec<Feature>) -> InputConfig {
        InputConfig {
            tag: Some(tag.to_owned()),
            manifest: None,
            channel: None,
            archs,
            variants: Vec::new(),
            features,
            sdk: None,
            crt: None,
        }
    }

    #[test]
    fn tag_validation() {
        assert!(validate_tag("17").is_ok());
        assert!(validate_tag("25H2").is_ok());
        assert!(validate_tag("my-tag_1.2").is_ok());
        assert!(validate_tag("").is_err());
        assert!(validate_tag(".").is_err());
        assert!(validate_tag("..").is_err());
        assert!(validate_tag("a/b").is_err());
        assert!(validate_tag("a\\b").is_err());
        assert!(validate_tag("a:b").is_err());
        assert!(validate_tag("a*b").is_err());
    }

    #[test]
    fn resolve_missing_tag_fails() {
        assert!(resolve_config(None, None, &Overrides::default(), &Defaults::default(), Some(Arch::X86_64), false).is_err());
    }

    #[test]
    fn resolve_tag_from_machine_defaults() {
        let defaults = Defaults {
            tag: Some("17".into()),
            ..Defaults::default()
        };
        let r = resolve_config(None, None, &Overrides::default(), &defaults, Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.tag, "17");
    }

    #[test]
    fn resolve_full() {
        let ov = Overrides {
            tag: Some("25H2".into()),
            positional_tag: None,
            manifest: Some(18),
            channel: Some("pre".into()),
            archs: Some(vec![Arch::X86_64, Arch::Aarch64]),
            variants: Some(vec![Variant::Desktop, Variant::OneCore]),
            features: Some(vec![Feature::Atl]),
            sdk: Some("10.0.28000.0".into()),
            crt: Some("14.40.33807".into()),
        };
        let r = resolve_config(None, None, &ov, &Defaults::default(), Some(Arch::X86), false).unwrap();
        assert_eq!(r.tag, "25H2");
        assert_eq!(r.manifest, 18);
        assert_eq!(r.channel, "pre");
        assert_eq!(r.archs, vec![Arch::X86_64, Arch::Aarch64]);
        assert_eq!(r.variants, vec![Variant::Desktop, Variant::OneCore]);
        assert_eq!(r.features, vec![Feature::Atl]);
        assert_eq!(r.sdk.as_deref(), Some("10.0.28000.0"));
    }

    #[test]
    fn resolve_union_adds_to_existing_tag() {
        // existing tag has x86_64, rerun adds x86 -> both
        let tag_cfg = input("17", vec![Arch::X86_64], vec![]);
        let ov = Overrides {
            tag: Some("17".into()),
            archs: Some(vec![Arch::X86]),
            features: Some(vec![Feature::Debug]),
            ..Overrides::default()
        };
        let r = resolve_config(Some(tag_cfg), None, &ov, &Defaults::default(), Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.archs, vec![Arch::X86_64, Arch::X86]);
        assert_eq!(r.features, vec![Feature::Debug]);
        assert_eq!(r.variants, vec![Variant::Desktop]);
    }

    #[test]
    fn resolve_machine_defaults_fill_only() {
        // machine defaults apply when nothing else specifies values
        let defaults = Defaults {
            archs: vec![Arch::X86_64, Arch::X86],
            features: vec![Feature::Atl],
            variants: vec![Variant::OneCore],
            manifest: Some(16),
            channel: Some("pre".into()),
            ..Defaults::default()
        };
        let ov = Overrides {
            tag: Some("17".into()),
            ..Overrides::default()
        };
        let r = resolve_config(None, None, &ov, &defaults, Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.archs, vec![Arch::X86_64, Arch::X86]);
        assert_eq!(r.features, vec![Feature::Atl]);
        assert_eq!(r.variants, vec![Variant::OneCore]);
        assert_eq!(r.manifest, 16);
        assert_eq!(r.channel, "pre");

        // explicit flags replace the defaults
        let ov = Overrides {
            tag: Some("17".into()),
            archs: Some(vec![Arch::Aarch64]),
            ..Overrides::default()
        };
        let r = resolve_config(None, None, &ov, &defaults, Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.archs, vec![Arch::Aarch64]);
        // ... but machine defaults still apply to the other lists
        assert_eq!(r.features, vec![Feature::Atl]);
    }

    #[test]
    fn resolve_scalar_precedence() {
        let tag_cfg = input("17", vec![], vec![]);
        let mut file_cfg = input("other", vec![], vec![]);
        file_cfg.sdk = Some("10.0.1.0".into());
        let ov = Overrides {
            tag: Some("17".into()),
            manifest: Some(16),
            ..Overrides::default()
        };
        let r = resolve_config(Some(tag_cfg), Some(file_cfg), &ov, &Defaults::default(), Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.manifest, 16);
        assert_eq!(r.sdk.as_deref(), Some("10.0.1.0"));
    }

    #[test]
    fn resolve_feature_default() {
        let ov = Overrides {
            tag: Some("17".into()),
            ..Overrides::default()
        };
        let r = resolve_config(None, None, &ov, &Defaults::default(), Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.features, vec![Feature::Debug]);
        let r = resolve_config(None, None, &ov, &Defaults::default(), Some(Arch::X86_64), true).unwrap();
        assert!(r.features.is_empty());
    }

    #[test]
    fn resolve_positional_tag() {
        let ov = Overrides {
            positional_tag: Some("16".into()),
            ..Overrides::default()
        };
        let r = resolve_config(None, None, &ov, &Defaults::default(), Some(Arch::X86_64), false).unwrap();
        assert_eq!(r.tag, "16");
    }

    #[test]
    fn input_config_roundtrip() {
        let cfg = SysrootConfig {
            tag: "17".into(),
            manifest: 17,
            channel: "release".into(),
            archs: vec![Arch::X86_64],
            variants: vec![Variant::Desktop],
            features: vec![Feature::Debug, Feature::Atl],
            sdk: "10.0.26100".into(),
            crt: "14.44.17.14".into(),
            vcr: Some("14.44.35211".into()),
        };
        let json = serde_json::to_value(&cfg).unwrap();
        assert_eq!(json["sdk"], "10.0.26100");
        assert_eq!(json["channel"], "release");
        assert!(json.get("vcr").is_some());

        let mut no_vcr = cfg.clone();
        no_vcr.vcr = None;
        let json = serde_json::to_value(&no_vcr).unwrap();
        assert!(json.get("vcr").is_none());

        let input: InputConfig = serde_json::from_value(json).unwrap();
        assert_eq!(input.tag.as_deref(), Some("17"));
        assert_eq!(input.channel.as_deref(), Some("release"));
        assert_eq!(input.archs, vec![Arch::X86_64]);
        assert_eq!(input.features, vec![Feature::Debug, Feature::Atl]);
    }

    #[test]
    fn machine_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save_json(&path, &MachineConfig::default()).unwrap();
        let loaded: MachineConfig = load_json(&path).unwrap().unwrap();
        assert_eq!(loaded, MachineConfig::default());

        // backward compat: old flat shape still parses
        let old = r#"{"cmake": true, "wine": false}"#;
        let old: MachineConfig = serde_json::from_str(old).unwrap();
        assert!(old.cmake);
        assert!(old.default.archs.is_empty());
    }

    #[test]
    fn machine_config_defaults_shape() {
        let json = r#"{
            "default": { "tag": "17", "archs": ["x86_64"], "manifest": 16, "channel": "pre" },
            "tpl": { "toolchain": "/tmp/custom.tpl" }
        }"#;
        let machine: MachineConfig = serde_json::from_str(json).unwrap();
        assert_eq!(machine.default.tag.as_deref(), Some("17"));
        assert_eq!(machine.default.archs, vec![Arch::X86_64]);
        assert_eq!(machine.default.manifest, Some(16));
        assert_eq!(machine.default.channel.as_deref(), Some("pre"));
        assert_eq!(machine.tpl.get("toolchain"), Some(&PathBuf::from("/tmp/custom.tpl")));
        assert!(!machine.cmake);
    }
}