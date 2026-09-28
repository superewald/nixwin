use crate::cmd::{Arch, Feature, Variant};
use crate::config::{self, MachineConfig, Overrides, SysrootConfig};
use crate::emit;
use crate::paths::{Paths, symlink_replace};
use crate::{Ctx, in_ci, payloads_to_work_items};
use anyhow::{Context as _, Result, ensure};
use indicatif::{MultiProgress, ProgressBar};
use std::path::{Path, PathBuf};
use xwin::util::ProgressTarget;
use xwin::{Ops, SplatConfig};

#[derive(Debug, Clone, clap::Args)]
pub struct InstallOptions {
    /// VS manifest version, shortcut for `--tag <V> --manifest <V>`
    pub manifest_version: Option<u8>,
    /// Name of the windows sysroot
    #[arg(short = 't', long)]
    pub tag: Option<String>,
    /// Target architectures to include
    #[arg(short = 'a', long, value_delimiter = ',')]
    pub archs: Vec<Arch>,
    /// Features to include
    #[arg(short = 'f', long, value_delimiter = ',')]
    pub features: Vec<Feature>,
    /// SDK/CRT variants to include
    #[arg(long, value_delimiter = ',')]
    pub variants: Vec<Variant>,
    /// Visual Studio manifest version to use
    #[arg(long)]
    pub manifest: Option<u8>,
    /// Visual Studio manifest channel to use, eg `release` or `pre`
    #[arg(long)]
    pub channel: Option<String>,
    /// Pin a specific WinSDK version. By default, resolves the latest version from the manifest
    #[arg(long)]
    pub sdk: Option<String>,
    /// Pin a specific CRT version to use. By default resolves the latest version from the manifest
    #[arg(long)]
    pub crt: Option<String>,
    /// Set this windows sysroot as the user default
    #[arg(short = 'd', long)]
    pub default: bool,
    /// Write a .nixwin.json lockfile for the installed sysroot. Optionally
    /// takes a file or project path, defaults to `$CWD/.nixwin.json`
    #[arg(short = 'l', long, num_args = 0..=1, default_missing_value = config::LOCKFILE)]
    pub lock: Option<PathBuf>,
}

/// Install (or update) a windows sysroot
pub fn install(opts: &InstallOptions, config_path: Option<&Path>, ctx: &Ctx) -> Result<()> {
    let (resolved, machine_config) = resolve_install_config(opts, config_path, ctx)?;

    let include_atl = resolved.features.contains(&Feature::Atl);
    let include_debug = resolved.features.contains(&Feature::Debug);

    // ---- xwin: manifest -> prune -> splat into the shared cache
    let xwin_ctx = ctx.xwin_ctx()?;
    let manifest = xwin::manifest::get_manifest(
        &xwin_ctx,
        resolved.manifest,
        &resolved.channel,
        ProgressBar::new(10),
    )?;
    let pkg_manifest =
        xwin::manifest::get_package_manifest(&xwin_ctx, &manifest, ProgressBar::new(10))?;

    let pruned = xwin::prune_pkg_list(
        &pkg_manifest,
        Arch::bits(&resolved.archs),
        Variant::bits(&resolved.variants),
        include_atl,
        include_debug,
        resolved.sdk.clone(),
        resolved.crt.clone(),
    )?;

    let arches = Arch::bits(&resolved.archs);
    let variants = Variant::bits(&resolved.variants);

    let dt = ProgressTarget::Stdout;
    let work_items = payloads_to_work_items(
        &pruned.payloads,
        MultiProgress::with_draw_target(dt.into()),
        dt,
    );

    let output: xwin::PathBuf = ctx
        .paths
        .cache_dir
        .to_str()
        .context("cache dir must be valid utf-8")?
        .to_owned()
        .into();

    xwin_ctx.execute(
        pkg_manifest.packages,
        work_items,
        pruned.crt_version.clone(),
        pruned.sdk_version.clone(),
        pruned.vcr_version.clone(),
        arches,
        variants,
        Ops::Splat(SplatConfig {
            include_debug_libs: include_debug,
            include_debug_symbols: include_debug,
            enable_symlinks: true,
            vfsoverlay: true,
            preserve_ms_arch_notation: true,
            use_winsysroot_style: true,
            preserve_versions: true,
            output,
            map: None,
            copy: false,
        }),
    )?;

    // ---- pinned sysroot configuration
    let cfg = SysrootConfig {
        tag: resolved.tag.clone(),
        manifest: resolved.manifest,
        channel: resolved.channel.clone(),
        archs: resolved.archs.clone(),
        variants: resolved.variants.clone(),
        features: resolved.features.clone(),
        sdk: pruned.sdk_version.clone(),
        crt: pruned.crt_version.clone(),
        vcr: pruned.vcr_version.clone(),
    };

    // ---- rebuild the tag view (symlinks into the cache) + integration files
    let tag_dir = ctx.paths.tag_dir(&cfg.tag);

    // capture the previous overlay before the tag directory is rebuilt, so
    // incremental feature installs keep their overlay entries
    let old_overlay = emit::vfs::load(&tag_dir.join("vfsoverlay.json"))?;

    link_tag(&tag_dir, &cfg, &ctx.paths)?;

    let cache_overlay = emit::vfs::load(&ctx.paths.cache_dir.join("vfsoverlay.json"))?;
    let merged = emit::vfs::merge(old_overlay, cache_overlay);
    emit::generate_all(&tag_dir, &cfg, merged.as_ref(), &machine_config.tpl)?;

    // ---- default handling
    let set_default = opts.default || ctx.paths.current_tag().is_none();
    if set_default {
        config::set_default_tag(ctx, &cfg.tag)?;
    }

    // ---- lockfile
    if let Some(lock_path) = &opts.lock {
        let lock = resolve_lock_path(lock_path);
        config::save_json(&lock, &cfg)
            .with_context(|| format!("unable to write lockfile {}", lock.display()))?;
        println!("lockfile: {}", lock.display());
    }

    println!("sysroot: {}", tag_dir.display());
    println!("crt: {}", cfg.crt);
    println!("sdk: {}", cfg.sdk);
    if let Some(vcr) = &cfg.vcr {
        println!("vcr: {vcr}");
    }
    if set_default {
        println!("default: {}", cfg.tag);
    }

    Ok(())
}

/// Resolves the `--lock` argument: a directory receives the default lockfile
/// name, any other path is used verbatim as the lockfile itself
fn resolve_lock_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join(config::LOCKFILE)
    } else {
        path.to_path_buf()
    }
}

fn resolve_install_config(
    opts: &InstallOptions,
    config_path: Option<&Path>,
    ctx: &Ctx,
) -> Result<(config::Resolved, MachineConfig)> {
    let machine_config = config::load_machine_config(ctx)?;

    let file_cfg_path = match config_path {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()
            .context("unable to determine current directory")?
            .join(config::LOCKFILE),
    };
    let file_cfg = config::load_input_config(&file_cfg_path)?;

    let positional = opts.manifest_version;

    let tag_hint = opts
        .tag
        .clone()
        .or_else(|| file_cfg.as_ref().and_then(|c| c.tag.clone()))
        .or_else(|| positional.map(|v| v.to_string()));

    anyhow::ensure!(
        tag_hint.is_some(),
        "missing sysroot tag, use --tag or the MANIFEST_VERSION argument"
    );
    let tag = tag_hint.unwrap();

    let tag_cfg = config::load_input_config(&ctx.paths.lockfile(&tag))?;

    let ov = Overrides {
        tag: opts.tag.clone(),
        positional_tag: positional.map(|v| v.to_string()),
        manifest: opts.manifest.or(positional),
        channel: opts.channel.clone(),
        archs: (!opts.archs.is_empty()).then(|| opts.archs.clone()),
        variants: (!opts.variants.is_empty()).then(|| opts.variants.clone()),
        features: (!opts.features.is_empty()).then(|| opts.features.clone()),
        sdk: opts.sdk.clone(),
        crt: opts.crt.clone(),
    };

    let resolved = config::resolve_config(
        tag_cfg,
        file_cfg,
        &ov,
        &machine_config.default,
        Arch::host(),
        in_ci(),
    )?;
    Ok((resolved, machine_config))
}

/// (Re)builds the sysroot view: directories of symlinks into the shared cache
pub(crate) fn link_tag(tag_dir: &Path, cfg: &SysrootConfig, paths: &Paths) -> Result<()> {
    if tag_dir.exists() {
        std::fs::remove_dir_all(tag_dir)
            .with_context(|| format!("unable to remove existing sysroot {}", tag_dir.display()))?;
    }
    std::fs::create_dir_all(tag_dir)
        .with_context(|| format!("unable to create sysroot {}", tag_dir.display()))?;

    // CRT: whole include dir + per-arch lib dirs (MS notation in the cache)
    let crt_cache = paths.cache_msvc.join(&cfg.crt);
    let crt_dir = tag_dir.join("VC/Tools/MSVC").join(&cfg.crt);
    std::fs::create_dir_all(crt_dir.join("lib"))?;
    ensure!(
        crt_cache.join("include").is_dir(),
        "CRT headers missing from cache: {}",
        crt_cache.join("include").display()
    );
    symlink_replace(&crt_cache.join("include"), &crt_dir.join("include"))?;
    for arch in &cfg.archs {
        let lib_src = crt_cache.join("lib").join(arch.as_ms_str());
        if lib_src.is_dir() {
            symlink_replace(&lib_src, &crt_dir.join("lib").join(arch.as_ms_str()))?;
        } else {
            eprintln!(
                "warning: CRT libs for {arch} missing in cache ({}), skipping",
                lib_src.display()
            );
        }
    }

    // SDK: whole Include/Lib version dirs
    let kit_dir = tag_dir.join("Windows Kits/10");
    let include_src = paths.cache_sdk.join("Include").join(&cfg.sdk);
    let lib_src = paths.cache_sdk.join("Lib").join(&cfg.sdk);
    ensure!(
        include_src.is_dir() && lib_src.is_dir(),
        "SDK missing from cache: {} / {}",
        include_src.display(),
        lib_src.display()
    );
    std::fs::create_dir_all(kit_dir.join("Include"))?;
    std::fs::create_dir_all(kit_dir.join("Lib"))?;
    symlink_replace(&include_src, &kit_dir.join("Include").join(&cfg.sdk))?;
    symlink_replace(&lib_src, &kit_dir.join("Lib").join(&cfg.sdk))?;

    // VCR debug libraries (only present with the debug feature)
    if let Some(vcr) = &cfg.vcr {
        for arch in &cfg.archs {
            let src = paths.cache_vcr.join(vcr).join("bin").join(arch.as_str());
            if src.is_dir() {
                let dst = tag_dir
                    .join("VCR")
                    .join(vcr)
                    .join("bin")
                    .join(arch.as_str());
                std::fs::create_dir_all(dst.parent().unwrap())?;
                symlink_replace(&src, &dst)?;
            } else {
                eprintln!(
                    "warning: VCR binaries for {arch} missing in cache ({}), skipping",
                    src.display()
                );
            }
        }
    }

    Ok(())
}

/// Resolves the tag a command should operate on: the explicit argument or the
/// current default
pub(crate) fn resolve_tag(tag: Option<&str>, ctx: &Ctx) -> Result<String> {
    if let Some(tag) = tag {
        config::validate_tag(tag)?;
        return Ok(tag.to_owned());
    }
    ctx.paths
        .current_tag()
        .context("no sysroot specified and no default sysroot set")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{Arch, Feature, Variant};

    fn cfg() -> SysrootConfig {
        SysrootConfig {
            tag: "17".into(),
            manifest: 17,
            channel: "release".into(),
            archs: vec![Arch::X86_64],
            variants: vec![Variant::Desktop],
            features: vec![Feature::Debug],
            sdk: "10.0.26100.0".into(),
            crt: "14.44.17.14".into(),
            vcr: Some("14.44.17.14".into()),
        }
    }

    #[test]
    fn resolve_lock_path_uses_lockfile_name_for_directories() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_lock_path(dir.path()),
            dir.path().join(".nixwin.json")
        );
    }

    #[test]
    fn resolve_lock_path_keeps_explicit_files() {
        assert_eq!(
            resolve_lock_path(Path::new("custom/my.lock.json")),
            PathBuf::from("custom/my.lock.json")
        );
    }

    /// Fake cache layout mimicking the xwin splat output
    fn fake_cache(paths: &Paths) {
        let crt = paths.cache_msvc.join("14.44.17.14");
        std::fs::create_dir_all(crt.join("include")).unwrap();
        for arch in ["x64", "x86"] {
            std::fs::create_dir_all(crt.join("lib").join(arch)).unwrap();
        }
        let sdk = &paths.cache_sdk;
        std::fs::create_dir_all(sdk.join("Include/10.0.26100.0")).unwrap();
        std::fs::create_dir_all(sdk.join("Lib/10.0.26100.0/um/x64")).unwrap();
        std::fs::create_dir_all(sdk.join("Lib/10.0.26100.0/ucrt/x64")).unwrap();
        let vcr = paths.cache_vcr.join("14.44.17.14");
        for arch in ["x86_64", "x86"] {
            std::fs::create_dir_all(vcr.join("bin").join(arch)).unwrap();
            std::fs::write(vcr.join("bin").join(arch).join("vcruntime140d.dll"), "x").unwrap();
        }
    }

    /// Fully isolated env: only HOME is set (ignores any real NIXWIN_* vars)
    fn isolated_env(home: &std::path::Path) -> impl Fn(&str) -> Option<String> + '_ {
        let home = home.display().to_string();
        move |key| (key == "HOME").then(|| home.clone())
    }

    #[test]
    fn link_tag_builds_symlink_view() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let env = isolated_env(&home);
        let paths = Paths::resolve_with(None, None, &env).unwrap();
        paths.create_dirs().unwrap();
        fake_cache(&paths);

        let tag_dir = paths.tag_dir("17");
        link_tag(&tag_dir, &cfg(), &paths).unwrap();

        // CRT
        assert!(tag_dir.join("VC/Tools/MSVC/14.44.17.14/include").is_dir());
        assert!(tag_dir.join("VC/Tools/MSVC/14.44.17.14/lib/x64").is_dir());
        // SDK
        assert!(
            tag_dir
                .join("Windows Kits/10/Include/10.0.26100.0")
                .is_dir()
        );
        assert!(tag_dir.join("Windows Kits/10/Lib/10.0.26100.0").is_dir());
        // VCR
        assert!(
            tag_dir
                .join("VCR/14.44.17.14/bin/x86_64/vcruntime140d.dll")
                .exists()
        );

        // links resolve into the cache
        let include_target =
            std::fs::read_link(tag_dir.join("VC/Tools/MSVC/14.44.17.14/include")).unwrap();
        assert_eq!(include_target, paths.cache_msvc.join("14.44.17.14/include"));

        // relinking (idempotent)
        link_tag(&tag_dir, &cfg(), &paths).unwrap();
        assert!(
            tag_dir
                .join("Windows Kits/10/Include/10.0.26100.0")
                .is_dir()
        );
    }

    #[test]
    fn link_tag_without_vcr() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let env = isolated_env(&home);
        let paths = Paths::resolve_with(None, None, &env).unwrap();
        paths.create_dirs().unwrap();
        fake_cache(&paths);

        let mut cfg = cfg();
        cfg.vcr = None;
        let tag_dir = paths.tag_dir("17");
        link_tag(&tag_dir, &cfg, &paths).unwrap();
        assert!(!tag_dir.join("VCR").exists());
    }
}
