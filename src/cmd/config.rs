use crate::Ctx;
use crate::cmd::{Arch, Feature, Variant, parse_csv_list};
use crate::config::{self, MachineConfig};
use crate::emit;
use anyhow::{Result, bail};
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Args)]
pub struct ConfigOptions {
    /// Config key: default.tag, default.arches, default.variants,
    /// default.features, default.manifest, default.channel, cmake, wine
    /// or tpl.<name>
    pub key: Option<String>,
    /// Value to set; omit to print the current value
    pub value: Option<String>,
    /// Remove the key from the configuration
    #[arg(long)]
    pub unset: bool,
}

/// Get or set values from the nixwin configuration (`$NIXWIN_DATA/config.json`)
pub fn config_cmd(opts: &ConfigOptions, ctx: &Ctx) -> Result<()> {
    if opts.unset && opts.value.is_some() {
        bail!("--unset cannot be combined with a value");
    }

    let path = ctx.paths.machine_config();
    let mut machine: MachineConfig = config::load_json(&path)?.unwrap_or_default();

    match &opts.key {
        None => {
            if opts.unset {
                bail!("--unset requires a key");
            }
            println!("{}", serde_json::to_string_pretty(&machine)?);
        }
        Some(key) => {
            let key = parse_key(key)?;
            if opts.unset {
                unset(&mut machine, key, ctx)?;
                config::save_json(&path, &machine)?;
            } else if let Some(value) = &opts.value {
                set(&mut machine, key, value, ctx)?;
                config::save_json(&path, &machine)?;
            } else {
                let value = get(&machine, &key, ctx)?;
                if !value.is_empty() {
                    println!("{value}");
                }
            }
        }
    }
    Ok(())
}

enum Key {
    DefaultTag,
    DefaultArches,
    DefaultVariants,
    DefaultFeatures,
    DefaultManifest,
    DefaultChannel,
    Cmake,
    Wine,
    Tpl(String),
}

fn parse_key(key: &str) -> Result<Key> {
    Ok(match key {
        "default.tag" => Key::DefaultTag,
        "default.arches" => Key::DefaultArches,
        "default.variants" => Key::DefaultVariants,
        "default.features" => Key::DefaultFeatures,
        "default.manifest" => Key::DefaultManifest,
        "default.channel" => Key::DefaultChannel,
        "cmake" => Key::Cmake,
        "wine" => Key::Wine,
        k if k.starts_with("tpl.") => {
            let name = &k["tpl.".len()..];
            anyhow::ensure!(
                emit::TEMPLATE_NAMES.contains(&name),
                "unknown template '{name}', expected one of: {}",
                emit::TEMPLATE_NAMES.join(", ")
            );
            Key::Tpl(name.to_owned())
        }
        _ => bail!(
            "unknown config key '{key}', expected one of: \
             default.tag, default.arches, default.variants, default.features, \
             default.manifest, default.channel, cmake, wine, \
             tpl.<{}>",
            emit::TEMPLATE_NAMES.join("/")
        ),
    })
}

fn set(machine: &mut MachineConfig, key: Key, value: &str, ctx: &Ctx) -> Result<()> {
    match key {
        Key::DefaultTag => {
            config::validate_tag(value)?;
            anyhow::ensure!(
                ctx.paths.tag_dir(value).is_dir(),
                "sysroot '{value}' is not installed"
            );
            machine.default.tag = Some(value.to_owned());
            ctx.paths.set_default(value)?;
            println!("{value}");
        }
        Key::DefaultArches => {
            machine.default.archs = parse_csv_list::<Arch>(value)?;
            println!(
                "{}",
                machine
                    .default
                    .archs
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        Key::DefaultVariants => {
            machine.default.variants = parse_csv_list::<Variant>(value)?;
            println!(
                "{}",
                machine
                    .default
                    .variants
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        Key::DefaultFeatures => {
            machine.default.features = parse_csv_list::<Feature>(value)?;
            println!(
                "{}",
                machine
                    .default
                    .features
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        Key::DefaultManifest => {
            machine.default.manifest = Some(value.parse().map_err(|_| {
                anyhow::anyhow!("manifest version must be a number, got '{value}'")
            })?);
            println!("{value}");
        }
        Key::DefaultChannel => {
            machine.default.channel = Some(value.to_owned());
            println!("{value}");
        }
        Key::Cmake => {
            machine.cmake = parse_bool(value)?;
            println!("{}", machine.cmake);
        }
        Key::Wine => {
            machine.wine = parse_bool(value)?;
            println!("{}", machine.wine);
        }
        Key::Tpl(name) => {
            let path = PathBuf::from(value);
            anyhow::ensure!(
                path.is_file(),
                "template '{}' does not exist",
                path.display()
            );
            machine.tpl.insert(name.clone(), path.clone());
            println!("{}", path.display());
        }
    }
    Ok(())
}

fn get(machine: &MachineConfig, key: &Key, ctx: &Ctx) -> Result<String> {
    Ok(match key {
        Key::DefaultTag => machine
            .default
            .tag
            .clone()
            .or_else(|| ctx.paths.current_tag())
            .unwrap_or_default(),
        Key::DefaultArches => machine
            .default
            .archs
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(","),
        Key::DefaultVariants => machine
            .default
            .variants
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(","),
        Key::DefaultFeatures => machine
            .default
            .features
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(","),
        Key::DefaultManifest => machine
            .default
            .manifest
            .map(|m| m.to_string())
            .unwrap_or_default(),
        Key::DefaultChannel => machine.default.channel.clone().unwrap_or_default(),
        Key::Cmake => machine.cmake.to_string(),
        Key::Wine => machine.wine.to_string(),
        Key::Tpl(name) => machine
            .tpl
            .get(name)
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    })
}

fn unset(machine: &mut MachineConfig, key: Key, ctx: &Ctx) -> Result<()> {
    match key {
        Key::DefaultTag => {
            machine.default.tag = None;
            // keep the default sysroot link in sync
            if ctx
                .paths
                .sysroot
                .symlink_metadata()
                .map(|m| m.is_symlink())
                .unwrap_or(false)
            {
                std::fs::remove_file(&ctx.paths.sysroot)?;
            }
        }
        Key::DefaultArches => machine.default.archs.clear(),
        Key::DefaultVariants => machine.default.variants.clear(),
        Key::DefaultFeatures => machine.default.features.clear(),
        Key::DefaultManifest => machine.default.manifest = None,
        Key::DefaultChannel => machine.default.channel = None,
        Key::Cmake => machine.cmake = false,
        Key::Wine => machine.wine = false,
        Key::Tpl(name) => {
            machine.tpl.remove(&name);
        }
    }
    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => bail!("invalid boolean '{value}', expected true/false"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MachineConfig;

    fn isolated_ctx() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let home_env = home.display().to_string();
        let env = move |key: &str| (key == "HOME").then(|| home_env.clone());
        let ctx = Ctx::with_env(None, None, &env).unwrap();
        (dir, ctx)
    }

    fn run_config(ctx: &Ctx, args: &[&str]) -> Result<()> {
        let unset = args.contains(&"--unset");
        let rest: Vec<&str> = args.iter().filter(|a| **a != "--unset").copied().collect();
        config_cmd(
            &ConfigOptions {
                key: rest.first().map(|s| s.to_string()),
                value: rest.get(1).map(|s| s.to_string()),
                unset,
            },
            ctx,
        )
    }

    fn machine(ctx: &Ctx) -> MachineConfig {
        config::load_json::<MachineConfig>(&ctx.paths.machine_config())
            .unwrap()
            .unwrap_or_default()
    }

    #[test]
    fn get_set_lists() {
        let (_dir, ctx) = isolated_ctx();

        // unset -> prints nothing
        run_config(&ctx, &["default.arches"]).unwrap();

        run_config(&ctx, &["default.arches", "x86,x86_64"]).unwrap();
        assert_eq!(machine(&ctx).default.archs, vec![Arch::X86, Arch::X86_64]);

        run_config(&ctx, &["default.features", "atl"]).unwrap();
        assert_eq!(machine(&ctx).default.features, vec![Feature::Atl]);

        run_config(&ctx, &["default.variants", "onecore,spectre"]).unwrap();
        assert_eq!(
            machine(&ctx).default.variants,
            vec![Variant::OneCore, Variant::Spectre]
        );

        // invalid value rejected
        assert!(run_config(&ctx, &["default.arches", "foo"]).is_err());
        assert!(run_config(&ctx, &["default.features", "wdk"]).is_err());
    }

    #[test]
    fn get_set_scalars() {
        let (_dir, ctx) = isolated_ctx();

        run_config(&ctx, &["default.manifest", "16"]).unwrap();
        assert_eq!(machine(&ctx).default.manifest, Some(16));

        run_config(&ctx, &["default.channel", "pre"]).unwrap();
        assert_eq!(machine(&ctx).default.channel.as_deref(), Some("pre"));

        run_config(&ctx, &["cmake", "true"]).unwrap();
        assert!(machine(&ctx).cmake);

        run_config(&ctx, &["wine", "false"]).unwrap();
        assert!(!machine(&ctx).wine);
        assert!(run_config(&ctx, &["wine", "maybe"]).is_err());
    }

    #[test]
    fn default_tag_syncs_symlink() {
        let (_dir, ctx) = isolated_ctx();

        // setting an uninstalled tag fails
        assert!(run_config(&ctx, &["default.tag", "17"]).is_err());

        std::fs::create_dir_all(ctx.paths.tag_dir("17")).unwrap();
        run_config(&ctx, &["default.tag", "17"]).unwrap();
        assert_eq!(machine(&ctx).default.tag.as_deref(), Some("17"));
        assert_eq!(ctx.paths.current_tag().as_deref(), Some("17"));

        // get falls back to the symlink when config is unset
        let mut m = machine(&ctx);
        m.default.tag = None;
        config::save_json(&ctx.paths.machine_config(), &m).unwrap();
        run_config(&ctx, &["default.tag"]).unwrap();
        assert_eq!(ctx.paths.current_tag().as_deref(), Some("17"));

        // unset removes both
        run_config(&ctx, &["default.tag", "--unset"]).unwrap();
        assert!(machine(&ctx).default.tag.is_none());
        assert!(ctx.paths.current_tag().is_none());
    }

    #[test]
    fn tpl_override() {
        let (_dir, ctx) = isolated_ctx();
        let tpl_dir = tempfile::tempdir().unwrap();
        let tpl = tpl_dir.path().join("custom.rustc.tpl");
        std::fs::write(&tpl, "x").unwrap();

        run_config(&ctx, &["tpl.toolchain", tpl.to_str().unwrap()]).unwrap();
        assert_eq!(machine(&ctx).tpl.get("toolchain"), Some(&tpl));

        // unknown template names rejected
        assert!(run_config(&ctx, &["tpl.nothing", "/tmp/x"]).is_err());

        // unset removes
        run_config(&ctx, &["tpl.toolchain", "--unset"]).unwrap();
        assert!(machine(&ctx).tpl.is_empty());
    }

    #[test]
    fn unknown_key_fails() {
        let (_dir, ctx) = isolated_ctx();
        assert!(run_config(&ctx, &["bogus.key"]).is_err());
        assert!(run_config(&ctx, &["tpl.nope", "/x"]).is_err());
    }
}
