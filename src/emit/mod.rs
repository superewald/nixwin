pub mod vfs;

pub use vfs::OverlayDoc;

use crate::cmd::Arch;
use crate::config::SysrootConfig;
use anyhow::{Context as _, Result};
use handlebars::Handlebars;
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::path::Path;

/// Builtin template sources: `(template name, contents)`
const TEMPLATES: [(&str, &str); 5] = [
    (
        "toolchain",
        include_str!("../../resources/templates/toolchain.cmake"),
    ),
    (
        "cmake",
        include_str!("../../resources/templates/cmake.env"),
    ),
    ("rustc", include_str!("../../resources/templates/rustc.env")),
    ("llvm", include_str!("../../resources/templates/llvm.env")),
    (
        "cmake-wrapper",
        include_str!("../../resources/templates/toolchain-wrapper.cmake"),
    ),
];

/// Valid template names, usable as `tpl.<name>` config keys
pub const TEMPLATE_NAMES: [&str; 5] = ["toolchain", "cmake", "rustc", "llvm", "cmake-wrapper"];

/// Builds a handlebars registry from the builtin templates, replacing any
/// source with a user override from `tpl.<name>` config entries.
fn registry(overrides: &BTreeMap<String, PathBuf>) -> Result<Handlebars<'static>> {
    let mut hb = Handlebars::new();
    // render paths and flags verbatim, no html escaping
    hb.register_escape_fn(handlebars::no_escape);
    // catch typos between templates and the render context
    hb.set_strict_mode(true);
    for (name, builtin) in TEMPLATES {
        let source = match overrides.get(name) {
            Some(path) => std::fs::read_to_string(path)
                .with_context(|| format!("unable to read template override {}", path.display()))?,
            None => builtin.to_owned(),
        };
        hb.register_template_string(name, source)?;
    }
    Ok(hb)
}

/// Renders the CMake toolchain wrapper (`setup --cmake`)
pub fn cmake_wrapper(tpl_overrides: &BTreeMap<String, PathBuf>) -> Result<String> {
    let hb = registry(tpl_overrides)?;
    Ok(hb.render("cmake-wrapper", &serde_json::json!({}))?)
}

/// The primary architecture of a sysroot (used as the toolchain default)
pub fn default_arch(archs: &[Arch]) -> Arch {
    *archs
        .iter()
        .min_by_key(|a| a.rank())
        .unwrap_or(&Arch::X86_64)
}

#[derive(serde::Serialize)]
struct TemplateCtx<'a> {
    tag: &'a str,
    sysroot: String,
    crt_version: &'a str,
    sdk_version: &'a str,
    default_arch: String,
    ms_arch: String,
}

impl<'a> TemplateCtx<'a> {
    fn new(cfg: &'a SysrootConfig, tag_dir: &Path) -> Self {
        let arch = default_arch(&cfg.archs);
        Self {
            tag: &cfg.tag,
            sysroot: tag_dir.display().to_string(),
            crt_version: &cfg.crt,
            sdk_version: &cfg.sdk,
            default_arch: arch.as_str().to_owned(),
            ms_arch: arch.as_ms_str().to_owned(),
        }
    }
}

/// Generates all per-sysroot integration files:
/// `vfsoverlay.json`, `toolchain.cmake`, `cmake.env`, `rustc.env`, `llvm.env`
/// and `nixwin.json`.
///
/// `vfsoverlay.json` and `nixwin.json` are emitted with serde_json (serialized
/// data), the remaining files are rendered from handlebars templates.
pub fn generate_all(
    tag_dir: &Path,
    cfg: &SysrootConfig,
    overlay: Option<&OverlayDoc>,
    tpl_overrides: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    let hb = registry(tpl_overrides)?;
    let ctx = TemplateCtx::new(cfg, tag_dir);

    vfs::write(&tag_dir.join("vfsoverlay.json"), overlay)?;
    std::fs::write(tag_dir.join("toolchain.cmake"), hb.render("toolchain", &ctx)?)?;

    for (file, name) in [
        ("cmake.env", "cmake"),
        ("rustc.env", "rustc"),
        ("llvm.env", "llvm"),
    ] {
        std::fs::write(tag_dir.join(file), hb.render(name, &ctx)?)?;
    }

    crate::config::save_json(&tag_dir.join("nixwin.json"), cfg)?;
    Ok(())
}

/// Renders an `inspect`-style summary of a sysroot config
pub fn inspect_output(cfg: &SysrootConfig) -> String {
    use crate::cmd::fmt_list;

    let mut out = String::new();
    out.push_str(&format!("tag: {}\n", cfg.tag));
    out.push_str(&format!("sdk: \"{}\"\n", cfg.sdk));
    out.push_str(&format!("crt: \"{}\"\n", cfg.crt));
    if let Some(vcr) = &cfg.vcr {
        out.push_str(&format!("vcr: \"{}\"\n", vcr));
    }
    out.push_str(&format!(
        "variants: {}\n",
        fmt_list(&cfg.variants.iter().map(|v| v.to_string()).collect::<Vec<_>>())
    ));
    out.push_str(&format!(
        "features: {}\n",
        fmt_list(&cfg.features.iter().map(|f| f.to_string()).collect::<Vec<_>>())
    ));
    out.push_str(&format!(
        "archs: {}",
        fmt_list(&cfg.archs.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{Arch, Feature, Variant};
    use crate::config::SysrootConfig;

    fn cfg() -> SysrootConfig {
        SysrootConfig {
            tag: "17".into(),
            manifest: 17,
            channel: "release".into(),
            archs: vec![Arch::X86_64, Arch::X86],
            variants: vec![Variant::Desktop],
            features: vec![Feature::Debug],
            sdk: "10.0.26100".into(),
            crt: "14.44.17.14".into(),
            vcr: Some("14.44.35211".into()),
        }
    }

    #[test]
    fn default_arch_prefers_x86_64() {
        assert_eq!(default_arch(&[Arch::X86, Arch::X86_64]), Arch::X86_64);
        assert_eq!(default_arch(&[Arch::Aarch64]), Arch::Aarch64);
        assert_eq!(default_arch(&[]), Arch::X86_64);
    }

    #[test]
    fn generates_all_files() {
        let dir = tempfile::tempdir().unwrap();
        let tag_dir = dir.path().join("sysroots/17");
        std::fs::create_dir_all(&tag_dir).unwrap();
        let overrides = BTreeMap::new();
        generate_all(&tag_dir, &cfg(), None, &overrides).unwrap();

        for name in [
            "vfsoverlay.json",
            "toolchain.cmake",
            "cmake.env",
            "rustc.env",
            "llvm.env",
            "nixwin.json",
        ] {
            assert!(tag_dir.join(name).exists(), "{name} missing");
        }

        // no leftover placeholders
        for name in ["toolchain.cmake", "cmake.env", "rustc.env", "llvm.env"] {
            let contents = std::fs::read_to_string(tag_dir.join(name)).unwrap();
            assert!(!contents.contains("{{"), "{name} has unrendered placeholders");
            assert!(!contents.contains("@CRT"), "{name} has unrendered placeholders");
        }

        // self-defaulting sysroot
        for name in ["cmake.env", "rustc.env", "llvm.env"] {
            let contents = std::fs::read_to_string(tag_dir.join(name)).unwrap();
            let expected = format!(
                "export NIXWIN_SYSROOT=\"${{NIXWIN_SYSROOT:-{}}}\"",
                tag_dir.display()
            );
            assert!(
                contents.starts_with(&expected),
                "{name} missing self-defaulting NIXWIN_SYSROOT export"
            );
        }

        // rustc.env: winsysroot + LIB
        let rustc_env = std::fs::read_to_string(tag_dir.join("rustc.env")).unwrap();
        assert!(rustc_env.contains("/winsysroot $NIXWIN_SYSROOT"));
        assert!(rustc_env.contains("export LIB=\"$NIXWIN_SYSROOT/VC/Tools/MSVC/14.44.17.14/lib/x64;"));
        assert!(rustc_env.contains("Windows Kits/10/Lib/10.0.26100/um/x64"));
        assert!(rustc_env.contains("RUSTFLAGS=\"-C linker=lld-link -C link-arg=/vfsoverlay:$NIXWIN_SYSROOT/vfsoverlay.json\""));

        // llvm.env: winsysroot + CFLAGS/CXXFLAGS
        let llvm_env = std::fs::read_to_string(tag_dir.join("llvm.env")).unwrap();
        assert!(llvm_env.contains("export NIXWIN_LLVM_FLAGS="));
        assert!(llvm_env.contains("export CFLAGS=$NIXWIN_LLVM_FLAGS"));
        assert!(llvm_env.contains("export CXXFLAGS=$NIXWIN_LLVM_FLAGS"));

        // cmake.env
        let cmake_env = std::fs::read_to_string(tag_dir.join("cmake.env")).unwrap();
        assert!(cmake_env.contains("CMAKE_TOOLCHAIN_FILE=$NIXWIN_SYSROOT/toolchain.cmake"));

        // toolchain.cmake: versions substituted
        let toolchain = std::fs::read_to_string(tag_dir.join("toolchain.cmake")).unwrap();
        assert!(toolchain.contains("set(NIXWIN_CRT_VERSION \"14.44.17.14\")"));
        assert!(toolchain.contains("set(NIXWIN_SDK_VERSION \"10.0.26100\")"));
        assert!(toolchain.contains("set(NIXWIN_TARGET_ARCH \"x86_64\")"));
        assert!(toolchain.contains(r#"/winsysroot \"${NIXWIN_SYSROOT}\""#));
        assert!(!toolchain.contains("vctoolsdir"));
    }

    #[test]
    fn template_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let tag_dir = dir.path().join("sysroots/17");
        std::fs::create_dir_all(&tag_dir).unwrap();

        let custom = dir.path().join("custom.rustc.tpl");
        std::fs::write(&custom, "custom {{crt_version}} {{ms_arch}} {{sysroot}}\n").unwrap();

        let mut overrides = BTreeMap::new();
        overrides.insert("rustc".to_owned(), custom.clone());

        generate_all(&tag_dir, &cfg(), None, &overrides).unwrap();
        let rendered = std::fs::read_to_string(tag_dir.join("rustc.env")).unwrap();
        assert_eq!(
            rendered,
            format!(
                "custom 14.44.17.14 x64 {}\n",
                tag_dir.display()
            )
        );
    }

    #[test]
    fn wrapper_renders() {
        let overrides = BTreeMap::new();
        let wrapper = cmake_wrapper(&overrides).unwrap();
        assert!(wrapper.contains("include(\"${_nixwin_sysroot}/toolchain.cmake\")"));
    }

    #[test]
    fn inspect_format() {
        let out = inspect_output(&cfg());
        assert!(out.starts_with("tag: 17\n"));
        assert!(out.contains("sdk: \"10.0.26100\""));
        assert!(out.contains("vcr: \"14.44.35211\""));
        assert!(out.contains("variants: [desktop]"));
        assert!(out.contains("features: [debug]"));
        assert!(out.contains("archs: [x86_64,x86]"));
    }
}