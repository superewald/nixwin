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
    /// Path to a nixwin.json configuration/lockfile
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
    /// Remove a windows sysroot from disk
    Rm { tag: Option<String> },
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
        Op::Rm { tag } => cmd::rm(tag.as_deref(), &ctx),
        Op::Inspect { tag } => cmd::inspect(tag.as_deref(), &ctx),
    }
}