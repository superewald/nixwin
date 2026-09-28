use clap::{Parser, Subcommand};
use nixwin::{Ctx, cmd};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to the nixwin cache directory ($NIXWIN_CACHE)
    #[arg(long, global = true)]
    cache_dir: Option<PathBuf>,
    /// Path to the nixwin data directory ($NIXWIN_DATA)
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Path to a .nixwin.json configuration/lockfile
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    op: Op,
}

#[derive(Debug, Subcommand)]
enum Op {
    /// Initialize nixwin on this machine (configuration and tool integration)
    Setup(cmd::SetupOptions),
    /// Install a pre-configured windows sysroot from the VS package store
    Install(cmd::InstallOptions),
    /// Get or set values from the nixwin configuration
    Config(cmd::ConfigOptions),
    /// List the installed windows sysroots
    Ls,
    /// Remove a windows sysroot, or single components from it
    ///
    /// Without component flags the whole sysroot is removed. With --arches,
    /// --features or --variants the named components are removed from the
    /// sysroot's configuration and its view is rebuilt.
    ///
    /// Removing an architecture or the debug feature reduces the sysroot on
    /// disk. Removing atl or a variant only updates the recorded configuration,
    /// which applies to future installs; it does not reduce the current sysroot.
    Rm(cmd::RmOptions),
    /// Print details about a windows sysroot
    Inspect { tag: Option<String> },
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    let ctx = Ctx::new(cli.data_dir, cli.cache_dir)?;

    match &cli.op {
        Op::Setup(opts) => cmd::setup(opts, &ctx),
        Op::Install(opts) => cmd::install(opts, cli.config.as_deref(), &ctx),
        Op::Config(opts) => cmd::config_cmd(opts, &ctx),
        Op::Ls => cmd::ls(&ctx),
        Op::Rm(opts) => cmd::rm(opts, &ctx),
        Op::Inspect { tag } => cmd::inspect(tag.as_deref(), &ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use nixwin::cmd::{Arch, Feature, Variant};

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("nixwin").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn rm_component_flags_parse_comma_separated_values() {
        let cli = parse(&[
            "rm",
            "17",
            "-a",
            "aarch64,x86",
            "-f",
            "debug",
            "--variants",
            "spectre",
        ]);
        let Op::Rm(opts) = &cli.op else {
            panic!("expected rm, got {cli:?}");
        };
        assert_eq!(opts.tag.as_deref(), Some("17"));
        assert_eq!(opts.arches, vec![Arch::Aarch64, Arch::X86]);
        assert_eq!(opts.features, vec![Feature::Debug]);
        assert_eq!(opts.variants, vec![Variant::Spectre]);
    }

    /// The component flags of `install` and `rm`, which must accept the same
    /// values
    struct Components {
        arches: Vec<Arch>,
        features: Vec<Feature>,
        variants: Vec<Variant>,
    }

    fn components(op: &Op) -> Components {
        match op {
            Op::Rm(opts) => Components {
                arches: opts.arches.clone(),
                features: opts.features.clone(),
                variants: opts.variants.clone(),
            },
            Op::Install(opts) => Components {
                arches: opts.archs.clone(),
                features: opts.features.clone(),
                variants: opts.variants.clone(),
            },
            other => panic!("expected install or rm, got {other:?}"),
        }
    }

    #[test]
    fn rm_component_flags_accept_the_same_values_as_install() {
        for value in ["x86", "x86_64", "aarch", "aarch64"] {
            assert_eq!(
                components(&parse(&["rm", "17", "-a", value]).op).arches,
                components(&parse(&["install", "17", "-a", value]).op).arches,
                "arch {value}"
            );
        }
        for value in ["debug", "atl"] {
            assert_eq!(
                components(&parse(&["rm", "17", "-f", value]).op).features,
                components(&parse(&["install", "17", "-f", value]).op).features,
                "feature {value}"
            );
        }
        for value in ["desktop", "onecore", "store", "spectre"] {
            assert_eq!(
                components(&parse(&["rm", "17", "--variants", value]).op).variants,
                components(&parse(&["install", "17", "--variants", value]).op).variants,
                "variant {value}"
            );
        }
    }

    #[test]
    fn rm_rejects_unknown_component_values() {
        for args in [
            ["rm", "17", "-a", "riscv"],
            ["rm", "17", "-f", "sanitizers"],
            ["rm", "17", "--variants", "uwp"],
        ] {
            assert!(
                Cli::try_parse_from(std::iter::once("nixwin").chain(args)).is_err(),
                "{args:?} should not parse"
            );
        }
    }

    #[test]
    fn rm_without_flags_keeps_whole_sysroot_semantics() {
        let cli = parse(&["rm", "17"]);
        let Op::Rm(opts) = &cli.op else {
            panic!("expected rm, got {cli:?}");
        };
        assert_eq!(opts.tag.as_deref(), Some("17"));
        assert!(opts.arches.is_empty());
        assert!(opts.features.is_empty());
        assert!(opts.variants.is_empty());
    }

    #[test]
    fn rm_help_mentions_the_future_installs_caveat() {
        let help = Cli::command()
            .find_subcommand_mut("rm")
            .unwrap()
            .render_long_help()
            .to_string();
        // the help wraps, so compare against whitespace-normalized text
        let help = help.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(help.contains("future installs"), "{help}");
        assert!(
            help.contains("does not reduce the current sysroot"),
            "{help}"
        );
    }
}
