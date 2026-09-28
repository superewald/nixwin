use crate::cmd::install::{link_tag, resolve_tag};
use crate::cmd::{Arch, Feature, Variant, fmt_list};
use crate::config::{self, MachineConfig, SysrootConfig};
use crate::{Ctx, emit};
use anyhow::{Context as _, Result, bail};
use std::fmt::Display;

#[derive(Debug, Clone, clap::Args)]
pub struct RmOptions {
    /// Name of the windows sysroot
    pub tag: Option<String>,
    /// Target architectures to remove
    #[arg(short = 'a', long = "arches", value_delimiter = ',')]
    pub arches: Vec<Arch>,
    /// Features to remove
    #[arg(short = 'f', long, value_delimiter = ',')]
    pub features: Vec<Feature>,
    /// SDK/CRT variants to remove
    #[arg(long, value_delimiter = ',')]
    pub variants: Vec<Variant>,
}

impl RmOptions {
    /// Whether any component was named on the command line
    fn has_components(&self) -> bool {
        !self.arches.is_empty() || !self.features.is_empty() || !self.variants.is_empty()
    }
}

/// List the installed windows sysroots
pub fn ls(ctx: &Ctx) -> Result<()> {
    let mut tags: Vec<String> = std::fs::read_dir(&ctx.paths.sysroots_dir)
        .with_context(|| format!("unable to read {}", ctx.paths.sysroots_dir.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry
                .path()
                .is_dir()
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    tags.sort();

    if tags.is_empty() {
        println!("no sysroots installed");
        return Ok(());
    }

    let default_tag = ctx.paths.current_tag();
    for tag in &tags {
        if default_tag.as_deref() == Some(tag) {
            println!("{tag} (default)");
        } else {
            println!("{tag}");
        }
    }
    Ok(())
}

/// Remove a windows sysroot from disk
pub fn rm(opts: &RmOptions, ctx: &Ctx) -> Result<()> {
    let tag = resolve_tag(opts.tag.as_deref(), ctx)?;
    let tag_dir = ctx.paths.tag_dir(&tag);
    anyhow::ensure!(
        tag_dir.is_dir(),
        "sysroot '{tag}' is not installed (expected {})",
        tag_dir.display()
    );

    if opts.has_components() {
        rm_components(&tag, opts, ctx)
    } else {
        rm_sysroot(&tag, ctx)
    }
}

/// Removes the whole sysroot, repairing the default sysroot link and the
/// configured default tag if they referenced it
fn rm_sysroot(tag: &str, ctx: &Ctx) -> Result<()> {
    let tag_dir = ctx.paths.tag_dir(tag);
    std::fs::remove_dir_all(&tag_dir)
        .with_context(|| format!("unable to remove {}", tag_dir.display()))?;

    // repair the default sysroot link if it pointed at the removed tag
    if ctx.paths.current_tag().as_deref() == Some(tag) {
        std::fs::remove_file(&ctx.paths.sysroot).with_context(|| {
            format!(
                "unable to remove default sysroot link {}",
                ctx.paths.sysroot.display()
            )
        })?;
    }

    let machine_path = ctx.paths.machine_config();
    if let Ok(Some(mut machine)) = config::load_json::<MachineConfig>(&machine_path)
        && machine.default.tag.as_deref() == Some(tag)
    {
        machine.default.tag = None;
        config::save_json(&machine_path, &machine)?;
    }

    println!("removed sysroot '{tag}'");
    Ok(())
}

/// Removes the named components from a sysroot's configuration and rebuilds
/// its view, in the same order `install` uses: `link_tag` (which wipes the
/// tag directory) then `emit::generate_all` (which re-creates the integration
/// files and writes `.nixwin.json`).
fn rm_components(tag: &str, opts: &RmOptions, ctx: &Ctx) -> Result<()> {
    let tag_dir = ctx.paths.tag_dir(tag);
    let lock = ctx.paths.lockfile(tag);
    let current: SysrootConfig = config::load_json(&lock)?
        .with_context(|| format!("sysroot '{tag}' has no .nixwin.json, is it installed?"))?;

    let (reduced, report) = subtract_components(tag, &current, opts)?;
    for line in &report {
        println!("{line}");
    }

    // the tag directory is wiped by link_tag, so read the overlay first
    let old_overlay = emit::vfs::load(&tag_dir.join("vfsoverlay.json"))?;
    link_tag(&tag_dir, &reduced, &ctx.paths)?;

    let machine = config::load_machine_config(ctx)?;
    let cache_overlay = emit::vfs::load(&ctx.paths.cache_dir.join("vfsoverlay.json"))?;
    let merged = emit::vfs::merge(old_overlay, cache_overlay);
    emit::generate_all(&tag_dir, &reduced, merged.as_ref(), &machine.tpl)?;

    println!("sysroot: {}", tag_dir.display());
    println!("archs: {}", fmt_list(&reduced.archs));
    Ok(())
}

/// Removes the named components from `cfg`, returning the reduced
/// configuration and the lines describing what was removed and what was not
/// present.
///
/// Architectures and the `debug` feature are expressible in the symlink view,
/// so removing them reduces the sysroot on disk. `atl` and variants are not:
/// their files are reached through wholesale-symlinked directories, so those
/// removals only take effect on the next install and the report says so.
fn subtract_components(
    tag: &str,
    cfg: &SysrootConfig,
    opts: &RmOptions,
) -> Result<(SysrootConfig, Vec<String>)> {
    let mut report = Vec::new();
    let mut reduced = cfg.clone();

    let arches = subtract(&cfg.archs, &opts.arches);
    report.extend(describe("removed arch:", &arches.removed));
    report.extend(describe("not present: arch", &arches.absent));
    // a sysroot without architectures links no libraries and cannot be used
    if arches.remaining.is_empty() {
        bail!("removing these arches would leave sysroot '{tag}' with no arches");
    }
    reduced.archs = arches.remaining;

    let features = subtract(&cfg.features, &opts.features);
    report.extend(describe("removed feature:", &features.removed));
    report.extend(describe("not present: feature", &features.absent));
    // the VCR tree is driven by the debug feature, so it only exists with it
    if features.removed.contains(&Feature::Debug) {
        reduced.vcr = None;
    }
    reduced.features = features.remaining;

    let variants = subtract(&cfg.variants, &opts.variants);
    report.extend(describe("removed variant:", &variants.removed));
    report.extend(describe("not present: variant", &variants.absent));
    reduced.variants = variants.remaining;

    if features.removed.contains(&Feature::Atl) || !variants.removed.is_empty() {
        report.push(
            "note: atl and variant removals are recorded for future installs and \
             do not reduce the files currently in the sysroot"
                .to_owned(),
        );
    }

    Ok((reduced, report))
}

/// The result of subtracting the named components from a recorded list
struct Subtraction<T> {
    /// the components that stay
    remaining: Vec<T>,
    /// the components that were removed
    removed: Vec<T>,
    /// the named components that were not present
    absent: Vec<T>,
}

fn subtract<T: Copy + PartialEq>(current: &[T], remove: &[T]) -> Subtraction<T> {
    Subtraction {
        remaining: current
            .iter()
            .copied()
            .filter(|item| !remove.contains(item))
            .collect(),
        removed: current
            .iter()
            .copied()
            .filter(|item| remove.contains(item))
            .collect(),
        absent: remove
            .iter()
            .copied()
            .filter(|item| !current.contains(item))
            .collect(),
    }
}

fn describe<T: Display>(prefix: &str, items: &[T]) -> Vec<String> {
    items
        .iter()
        .map(|item| format!("{prefix} {item}"))
        .collect()
}

/// Print details about a windows sysroot
pub fn inspect(tag: Option<&str>, ctx: &Ctx) -> Result<()> {
    let tag = resolve_tag(tag, ctx)?;
    let cfg: SysrootConfig = config::load_json(&ctx.paths.lockfile(&tag))?
        .with_context(|| format!("sysroot '{tag}' has no .nixwin.json, is it installed?"))?;
    println!("{}", emit::inspect_output(&cfg));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Paths;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    const CRT: &str = "14.44.17.14";
    const SDK: &str = "10.0.26100.0";

    fn cfg() -> SysrootConfig {
        SysrootConfig {
            tag: "17".into(),
            manifest: 17,
            channel: "release".into(),
            archs: arches(),
            variants: vec![Variant::Desktop, Variant::Spectre],
            features: features(),
            sdk: SDK.into(),
            crt: CRT.into(),
            vcr: Some(CRT.into()),
        }
    }

    fn arches() -> Vec<Arch> {
        vec![Arch::X86_64, Arch::X86, Arch::Aarch64]
    }

    fn features() -> Vec<Feature> {
        vec![Feature::Debug, Feature::Atl]
    }

    fn opts(arches: Vec<Arch>, features: Vec<Feature>, variants: Vec<Variant>) -> RmOptions {
        RmOptions {
            tag: Some("17".into()),
            arches,
            features,
            variants,
        }
    }

    /// Fake cache layout mimicking the xwin splat output
    fn fake_cache(paths: &Paths) {
        let crt = paths.cache_msvc.join(CRT);
        std::fs::create_dir_all(crt.join("include")).unwrap();
        for arch in ["x64", "x86", "arm64", "arm"] {
            std::fs::create_dir_all(crt.join("lib").join(arch)).unwrap();
        }
        let sdk = &paths.cache_sdk;
        std::fs::create_dir_all(sdk.join("Include").join(SDK)).unwrap();
        for arch in ["x64", "x86", "arm64", "arm"] {
            std::fs::create_dir_all(sdk.join("Lib").join(SDK).join("um").join(arch)).unwrap();
            std::fs::create_dir_all(sdk.join("Lib").join(SDK).join("ucrt").join(arch)).unwrap();
        }
        let vcr = paths.cache_vcr.join(CRT);
        for arch in ["x86_64", "x86", "aarch64", "aarch"] {
            std::fs::create_dir_all(vcr.join("bin").join(arch)).unwrap();
        }
    }

    /// Fully isolated env: only HOME is set (ignores any real NIXWIN_* vars)
    fn isolated_env(home: &std::path::Path) -> impl Fn(&str) -> Option<String> + '_ {
        let home = home.display().to_string();
        move |key| (key == "HOME").then(|| home.clone())
    }

    /// An installed sysroot with a populated cache, as `rm` would find it
    fn installed() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        let env = isolated_env(&home);
        let ctx = Ctx::with_env(None, None, &env).unwrap();
        fake_cache(&ctx.paths);

        let tag_dir = ctx.paths.tag_dir("17");
        std::fs::create_dir_all(&tag_dir).unwrap();
        let cfg = cfg();
        link_tag(&tag_dir, &cfg, &ctx.paths).unwrap();
        emit::generate_all(&tag_dir, &cfg, None, &BTreeMap::new()).unwrap();
        (dir, ctx)
    }

    fn crt_lib(tag_dir: &Path, arch: Arch) -> PathBuf {
        tag_dir
            .join("VC/Tools/MSVC")
            .join(CRT)
            .join("lib")
            .join(arch.as_ms_str())
    }

    fn vcr(tag_dir: &Path, arch: Arch) -> PathBuf {
        tag_dir
            .join("VCR")
            .join(CRT)
            .join("bin")
            .join(arch.as_str())
    }

    /// The integration files `emit::generate_all` writes into a sysroot
    const INTEGRATION_FILES: [&str; 5] = [
        "llvm.env",
        "cmake.env",
        "rustc.env",
        "toolchain.cmake",
        "vfsoverlay.json",
    ];

    #[test]
    fn rm_without_components_removes_the_whole_sysroot() {
        let (_dir, ctx) = installed();
        let machine_path = ctx.paths.machine_config();
        let mut machine = MachineConfig::default();
        machine.default.tag = Some("17".into());
        config::save_json(&machine_path, &machine).unwrap();
        ctx.paths.set_default("17").unwrap();

        rm(&opts(Vec::new(), Vec::new(), Vec::new()), &ctx).unwrap();

        assert!(!ctx.paths.tag_dir("17").exists());
        // both repairs fire: the link is gone and the default tag is cleared
        assert!(ctx.paths.current_tag().is_none());
        let machine: MachineConfig = config::load_json(&machine_path).unwrap().unwrap();
        assert_eq!(machine.default.tag, None);
    }

    #[test]
    fn rm_leaves_other_sysroots_alone_when_removing_a_default() {
        let (_dir, ctx) = installed();

        // a second sysroot, made default
        let other = ctx.paths.tag_dir("16");
        std::fs::create_dir_all(&other).unwrap();
        ctx.paths.set_default("16").unwrap();
        config::set_default_tag(&ctx, "16").unwrap();

        rm(
            &RmOptions {
                tag: Some("17".into()),
                ..opts(Vec::new(), Vec::new(), Vec::new())
            },
            &ctx,
        )
        .unwrap();

        assert!(!ctx.paths.tag_dir("17").exists());
        assert!(other.is_dir());
        assert_eq!(ctx.paths.current_tag().as_deref(), Some("16"));
        let machine: MachineConfig = config::load_json(&ctx.paths.machine_config())
            .unwrap()
            .unwrap();
        assert_eq!(machine.default.tag.as_deref(), Some("16"));
    }

    #[test]
    fn rm_errors_on_an_absent_sysroot() {
        let (_dir, ctx) = installed();
        let err = rm(
            &RmOptions {
                tag: Some("99".into()),
                ..opts(Vec::new(), Vec::new(), Vec::new())
            },
            &ctx,
        )
        .unwrap_err();
        assert!(err.to_string().contains("not installed"), "{err}");
        // untouched
        assert!(ctx.paths.tag_dir("17").is_dir());
    }

    #[test]
    fn rm_architecture_drops_its_lib_dir_and_keeps_the_rest() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");
        assert!(crt_lib(&tag_dir, Arch::Aarch64).is_dir());

        rm(&opts(vec![Arch::Aarch64], Vec::new(), Vec::new()), &ctx).unwrap();

        // the removed architecture's libs are gone from the view
        assert!(!crt_lib(&tag_dir, Arch::Aarch64).exists());
        assert!(!vcr(&tag_dir, Arch::Aarch64).exists());
        // the remaining ones are intact and still resolve into the cache
        for arch in [Arch::X86_64, Arch::X86] {
            assert!(crt_lib(&tag_dir, arch).is_dir(), "{arch} libs missing");
            assert!(vcr(&tag_dir, arch).is_dir(), "{arch} VCR missing");
            let target = std::fs::read_link(crt_lib(&tag_dir, arch)).unwrap();
            assert_eq!(
                target,
                ctx.paths
                    .cache_msvc
                    .join(CRT)
                    .join("lib")
                    .join(arch.as_ms_str())
            );
        }
        // the recorded configuration reflects the subtraction
        let after: SysrootConfig = config::load_json(&ctx.paths.lockfile("17"))
            .unwrap()
            .unwrap();
        assert_eq!(after.archs, vec![Arch::X86_64, Arch::X86]);
    }

    #[test]
    fn rm_debug_feature_removes_the_vcr_dir() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");
        assert!(tag_dir.join("VCR").is_dir());

        rm(&opts(Vec::new(), vec![Feature::Debug], Vec::new()), &ctx).unwrap();

        assert!(!tag_dir.join("VCR").exists());
        // the rest of the view is untouched
        assert!(crt_lib(&tag_dir, Arch::X86_64).is_dir());
        let after: SysrootConfig = config::load_json(&ctx.paths.lockfile("17"))
            .unwrap()
            .unwrap();
        assert_eq!(after.features, vec![Feature::Atl]);
        assert_eq!(after.vcr, None);
    }

    #[test]
    fn rm_keeps_integration_files_and_updates_them() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");

        rm(
            &opts(vec![Arch::Aarch64], vec![Feature::Debug], Vec::new()),
            &ctx,
        )
        .unwrap();

        // a rebuild that forgot emit::generate_all would leave none of these
        for file in INTEGRATION_FILES {
            assert!(tag_dir.join(file).exists(), "{file} missing after removal");
        }
        // and they reflect the reduced configuration
        let toolchain = std::fs::read_to_string(tag_dir.join("toolchain.cmake")).unwrap();
        assert!(toolchain.contains(r#"set(NIXWIN_TARGET_ARCH "x86_64")"#));
        let rustc_env = std::fs::read_to_string(tag_dir.join("rustc.env")).unwrap();
        assert!(rustc_env.contains("lib/x64;"));
        assert!(!rustc_env.contains("lib/arm64;"));
    }

    #[test]
    fn rm_keeps_the_default_link_resolving() {
        let (_dir, ctx) = installed();
        ctx.paths.set_default("17").unwrap();

        rm(&opts(vec![Arch::Aarch64], Vec::new(), Vec::new()), &ctx).unwrap();

        // the tag itself remains, so the default link still resolves
        assert_eq!(ctx.paths.current_tag().as_deref(), Some("17"));
        assert!(ctx.paths.sysroot.join("llvm.env").exists());
    }

    #[test]
    fn rm_several_components_at_once() {
        let (_dir, ctx) = installed();
        rm(
            &opts(
                vec![Arch::Aarch64],
                vec![Feature::Atl],
                vec![Variant::Spectre],
            ),
            &ctx,
        )
        .unwrap();

        let after: SysrootConfig = config::load_json(&ctx.paths.lockfile("17"))
            .unwrap()
            .unwrap();
        assert_eq!(after.archs, vec![Arch::X86_64, Arch::X86]);
        assert_eq!(after.features, vec![Feature::Debug]);
        assert_eq!(after.variants, vec![Variant::Desktop]);

        // ... which is what `nixwin inspect` reads back
        let out = emit::inspect_output(&after);
        assert!(out.contains("archs: [x86_64,x86]"), "{out}");
        assert!(out.contains("features: [debug]"), "{out}");
        assert!(out.contains("variants: [desktop]"), "{out}");
    }

    #[test]
    fn rm_atl_and_variants_do_not_reduce_the_view() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");
        let include = tag_dir.join("VC/Tools/MSVC").join(CRT).join("include");

        let (reduced, report) = subtract_components(
            "17",
            &cfg(),
            &opts(Vec::new(), vec![Feature::Atl], vec![Variant::Spectre]),
        )
        .unwrap();

        assert_eq!(reduced.features, vec![Feature::Debug]);
        assert_eq!(reduced.variants, vec![Variant::Desktop]);
        // the report states the change is for future installs
        assert!(
            report.iter().any(|l| l.contains("future installs")),
            "{report:?}"
        );

        // the wholesale-symlinked include dir is not expressible per feature
        assert!(include.is_dir());
        rm(&opts(Vec::new(), vec![Feature::Atl], Vec::new()), &ctx).unwrap();
        assert!(include.is_dir());
    }

    #[test]
    fn rm_architecture_removal_does_not_print_the_future_installs_note() {
        let (_, report) = subtract_components(
            "17",
            &cfg(),
            &opts(vec![Arch::Aarch64], Vec::new(), Vec::new()),
        )
        .unwrap();
        assert!(
            report.contains(&"removed arch: aarch64".to_owned()),
            "{report:?}"
        );
        assert!(
            !report.iter().any(|l| l.contains("future installs")),
            "{report:?}"
        );
    }

    #[test]
    fn rm_rejects_emptying_the_architecture_list() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");
        let before = std::fs::read_to_string(ctx.paths.lockfile("17")).unwrap();

        let err = rm(&opts(arches(), Vec::new(), Vec::new()), &ctx).unwrap_err();
        assert!(
            err.to_string().contains("no arches"),
            "unexpected error: {err}"
        );

        // the sysroot is left unchanged and still inspectable
        let after: SysrootConfig = config::load_json(&ctx.paths.lockfile("17"))
            .unwrap()
            .unwrap();
        assert_eq!(after.archs, arches());
        assert_eq!(
            std::fs::read_to_string(ctx.paths.lockfile("17")).unwrap(),
            before
        );
        for file in INTEGRATION_FILES {
            assert!(tag_dir.join(file).exists(), "{file} missing");
        }
        assert!(crt_lib(&tag_dir, Arch::X86_64).is_dir());
        let out = emit::inspect_output(&after);
        assert!(out.contains("archs: [x86_64,x86,aarch64]"), "{out}");
    }

    #[test]
    fn rm_reports_absent_components_without_failing() {
        let (_dir, ctx) = installed();

        // aarch is not in the sysroot, spectre and onecore are not recorded
        let err = rm(
            &opts(
                vec![Arch::Aarch, Arch::X86_64],
                Vec::new(),
                vec![Variant::OneCore],
            ),
            &ctx,
        );
        assert!(err.is_ok(), "{err:?}");

        // the absent components are reported, the present one is removed
        let after: SysrootConfig = config::load_json(&ctx.paths.lockfile("17"))
            .unwrap()
            .unwrap();
        assert_eq!(after.archs, vec![Arch::X86, Arch::Aarch64]);

        let (_, report) = subtract_components(
            "17",
            &cfg(),
            &opts(
                vec![Arch::Aarch, Arch::X86_64],
                Vec::new(),
                vec![Variant::OneCore],
            ),
        )
        .unwrap();
        assert!(
            report.contains(&"not present: arch aarch".to_owned()),
            "{report:?}"
        );
        assert!(
            report.contains(&"not present: variant onecore".to_owned()),
            "{report:?}"
        );
    }

    #[test]
    fn rm_absent_architecture_is_reported_not_an_error() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");

        // only an absent architecture: nothing changes, nothing breaks
        rm(&opts(vec![Arch::Aarch], Vec::new(), Vec::new()), &ctx).unwrap();

        let after: SysrootConfig = config::load_json(&ctx.paths.lockfile("17"))
            .unwrap()
            .unwrap();
        assert_eq!(after.archs, arches());
        for arch in arches() {
            assert!(crt_lib(&tag_dir, arch).is_dir(), "{arch} libs missing");
        }
    }

    #[test]
    fn rm_is_safe_to_re_run() {
        let (_dir, ctx) = installed();
        let first = opts(
            vec![Arch::Aarch64],
            vec![Feature::Atl],
            vec![Variant::Spectre],
        );

        rm(&first, &ctx).unwrap();
        let after_first = std::fs::read_to_string(ctx.paths.lockfile("17")).unwrap();

        // the same command again: every component is already gone
        rm(&first, &ctx).unwrap();
        assert_eq!(
            std::fs::read_to_string(ctx.paths.lockfile("17")).unwrap(),
            after_first
        );

        let (_, report) = subtract_components(
            "17",
            &config::load_json::<SysrootConfig>(&ctx.paths.lockfile("17"))
                .unwrap()
                .unwrap(),
            &first,
        )
        .unwrap();
        assert!(
            report.contains(&"not present: arch aarch64".to_owned()),
            "{report:?}"
        );
        assert!(
            report.contains(&"not present: feature atl".to_owned()),
            "{report:?}"
        );
        assert!(
            report.contains(&"not present: variant spectre".to_owned()),
            "{report:?}"
        );
    }

    #[test]
    fn rm_preserves_the_existing_vfs_overlay() {
        let (_dir, ctx) = installed();
        let tag_dir = ctx.paths.tag_dir("17");
        let overlay = emit::OverlayDoc {
            roots: vec![emit::vfs::Entry::DirectoryRemap {
                name: "atl/atlbase.h".into(),
                external_contents: "atl/base.h".into(),
            }],
            ..emit::OverlayDoc::default()
        };
        emit::vfs::write(&tag_dir.join("vfsoverlay.json"), Some(&overlay)).unwrap();

        rm(&opts(vec![Arch::Aarch64], Vec::new(), Vec::new()), &ctx).unwrap();

        let merged = emit::vfs::load(&tag_dir.join("vfsoverlay.json"))
            .unwrap()
            .unwrap();
        assert_eq!(merged, overlay);
    }
}
