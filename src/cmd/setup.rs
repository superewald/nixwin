use crate::config::{self, MachineConfig, SysrootConfig};
use crate::cmd::Arch;
use crate::emit;
use crate::Ctx;
use anyhow::{Context as _, Result};

#[derive(Debug, Clone, clap::Args)]
pub struct SetupOptions {
    /// Configure the CMake integration: write a toolchain wrapper which
    /// auto-selects the sysroot toolchain when a `nixwin.json` is present,
    /// and persist `CMAKE_TOOLCHAIN_FILE` in your shell rc
    #[arg(long)]
    pub cmake: bool,
    /// Add the VCR debug libraries to your user wine prefix(es)
    #[arg(long)]
    pub wine: bool,
    /// Wine prefix to install into (defaults to `$WINEPREFIX` or `~/.wine`)
    #[arg(long)]
    pub wine_prefix: Option<std::path::PathBuf>,
}

/// Initialize nixwin on the machine, create configuration and tool integration
pub fn setup(opts: &SetupOptions, ctx: &Ctx) -> Result<()> {
    let config_path = ctx.paths.machine_config();
    let mut machine: MachineConfig = config::load_json(&config_path)?.unwrap_or_default();

    if opts.cmake {
        machine.cmake = true;
        setup_cmake(ctx, &machine.tpl)?;
    }

    if opts.wine {
        machine.wine = true;
        setup_wine(ctx, opts.wine_prefix.clone())?;
    }

    config::save_json(&config_path, &machine)?;

    if !opts.cmake && !opts.wine {
        println!("nixwin initialized");
        println!("data:  {}", ctx.paths.data_dir.display());
        println!("cache: {}", ctx.paths.cache_dir.display());
        println!();
        println!("install a windows sysroot with `nixwin install 17 --default`");
        println!("available integrations: `nixwin setup --cmake --wine`");
    }

    Ok(())
}

fn shell_rc() -> Option<std::path::PathBuf> {
    let home = std::env::home_dir()?;
    let shell = std::env::var("SHELL").ok()?;
    if shell.ends_with("zsh") {
        Some(home.join(".zshrc"))
    } else if shell.ends_with("bash") {
        Some(home.join(".bashrc"))
    } else if home.join(".zshrc").exists() {
        Some(home.join(".zshrc"))
    } else {
        Some(home.join(".bashrc"))
    }
}

const CMAKE_RC_MARKER: &str = "# >>> nixwin (cmake) >>>";

fn setup_cmake(ctx: &Ctx, tpl_overrides: &std::collections::BTreeMap<String, std::path::PathBuf>) -> Result<()> {
    let wrapper = ctx.paths.data_dir.join("toolchain.cmake");
    std::fs::write(&wrapper, emit::cmake_wrapper(tpl_overrides)?)
        .with_context(|| format!("unable to write {}", wrapper.display()))?;
    println!("cmake wrapper: {}", wrapper.display());

    match shell_rc() {
        Some(rc) => {
            let export = format!("export CMAKE_TOOLCHAIN_FILE=\"{}\"", wrapper.display());
            let block = format!(
                "{CMAKE_RC_MARKER}\n{export}\n# <<< nixwin (cmake) <<<\n"
            );

            let contents = std::fs::read_to_string(&rc).unwrap_or_default();
            if contents.contains(CMAKE_RC_MARKER) {
                println!("shell rc: {} (nixwin block already present)", rc.display());
            } else {
                let mut updated = contents;
                if !updated.is_empty() && !updated.ends_with('\n') {
                    updated.push('\n');
                }
                updated.push_str(&block);
                std::fs::write(&rc, updated)
                    .with_context(|| format!("unable to write {}", rc.display()))?;
                println!("shell rc: {} (added CMAKE_TOOLCHAIN_FILE export)", rc.display());
            }
        }
        None => {
            println!(
                "could not detect your shell rc, add to your profile:\n  export CMAKE_TOOLCHAIN_FILE=\"{}\"",
                wrapper.display()
            );
        }
    }

    Ok(())
}

fn setup_wine(
    ctx: &Ctx,
    wine_prefix: Option<std::path::PathBuf>,
) -> Result<()> {
    let prefix = wine_prefix
        .or_else(|| std::env::var("WINEPREFIX").ok().map(std::path::PathBuf::from))
        .or_else(|| std::env::home_dir().map(|home| home.join(".wine")))
        .context("unable to determine wine prefix, specify it with --wine-prefix")?;

    let tag = crate::cmd::resolve_tag(None, ctx)?;
    let cfg: SysrootConfig = config::load_json(&ctx.paths.tag_dir(&tag).join("nixwin.json"))?
        .with_context(|| format!("sysroot '{tag}' has no nixwin.json, is it installed?"))?;

    let vcr = cfg.vcr.as_ref().context(
        "the default sysroot has no VCR debug libraries, install it with --features debug",
    )?;

    let mut installed = 0usize;
    for arch in &cfg.archs {
        // 64-bit dlls go to system32, 32-bit dlls to syswow64
        let dst_name = match arch {
            Arch::X86_64 | Arch::Aarch64 => "system32",
            Arch::X86 | Arch::Aarch => "syswow64",
        };
        let src = ctx.paths.cache_vcr.join(vcr).join("bin").join(arch.as_str());
        if !src.is_dir() {
            eprintln!("warning: VCR binaries for {arch} missing in cache ({}), skipping", src.display());
            continue;
        }
        let dst = prefix.join("drive_c/windows").join(dst_name);
        std::fs::create_dir_all(&dst)
            .with_context(|| format!("unable to create {}", dst.display()))?;

        for entry in std::fs::read_dir(&src)
            .with_context(|| format!("unable to read {}", src.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("dll") {
                continue;
            }
            let file_name = entry.file_name();
            std::fs::copy(&path, dst.join(&file_name))
                .with_context(|| format!("unable to install {}", path.display()))?;
            installed += 1;
        }
    }

    println!("{installed} debug dll(s) installed into {}", prefix.display());
    Ok(())
}
