use anyhow::{Context as _, Result};
use std::path::{Path, PathBuf};

/// Injectable environment lookup (key -> value), for tests
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

pub fn real_env() -> impl Fn(&str) -> Option<String> {
    |key| std::env::var(key).ok().filter(|v| !v.is_empty())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// `$NIXWIN_DATA`, default `$HOME/.local/share/nixwin`
    pub data_dir: PathBuf,
    /// `$NIXWIN_CACHE`, default `$HOME/.cache/nixwin`
    pub cache_dir: PathBuf,
    /// `$NIXWIN_SYSROOTS`, default `$NIXWIN_DATA/sysroots`
    pub sysroots_dir: PathBuf,
    /// `$NIXWIN_SYSROOT`, default `$NIXWIN_DATA/sysroot` (symlink to the default tag)
    pub sysroot: PathBuf,
    /// `$NIXWIN_CACHE_MSVC`, default `$NIXWIN_CACHE/VC/Tools/MSVC`
    pub cache_msvc: PathBuf,
    /// `$NIXWIN_CACHE_SDK`, default `$NIXWIN_CACHE/Windows Kits/10`
    pub cache_sdk: PathBuf,
    /// `$NIXWIN_CACHE_VCR`, default `$NIXWIN_CACHE/VCR`
    pub cache_vcr: PathBuf,
}

impl Paths {
    pub fn resolve(data_dir: Option<PathBuf>, cache_dir: Option<PathBuf>) -> Result<Self> {
        let env = real_env();
        Self::resolve_with(data_dir, cache_dir, &env)
    }

    pub fn resolve_with(
        data_dir: Option<PathBuf>,
        cache_dir: Option<PathBuf>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self> {
        let home = env("HOME")
            .map(PathBuf::from)
            .or_else(std::env::home_dir)
            .context("failed to resolve home directory")?;

        let data_dir = data_dir
            .or_else(|| env("NIXWIN_DATA").map(PathBuf::from))
            .unwrap_or_else(|| home.join(".local/share/nixwin"));

        let cache_dir = cache_dir
            .or_else(|| env("NIXWIN_CACHE").map(PathBuf::from))
            .unwrap_or_else(|| home.join(".cache/nixwin"));

        Ok(Self {
            sysroots_dir: env("NIXWIN_SYSROOTS")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir.join("sysroots")),
            sysroot: env("NIXWIN_SYSROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| data_dir.join("sysroot")),
            cache_msvc: env("NIXWIN_CACHE_MSVC")
                .map(PathBuf::from)
                .unwrap_or_else(|| cache_dir.join("VC/Tools/MSVC")),
            cache_sdk: env("NIXWIN_CACHE_SDK")
                .map(PathBuf::from)
                .unwrap_or_else(|| cache_dir.join("Windows Kits/10")),
            cache_vcr: env("NIXWIN_CACHE_VCR")
                .map(PathBuf::from)
                .unwrap_or_else(|| cache_dir.join("VCR")),
            data_dir,
            cache_dir,
        })
    }

    pub fn create_dirs(&self) -> Result<()> {
        for dir in [&self.data_dir, &self.cache_dir, &self.sysroots_dir] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("unable to create directory {}", dir.display()))?;
        }
        Ok(())
    }

    /// Directory of the sysroot with the given tag
    pub fn tag_dir(&self, tag: &str) -> PathBuf {
        self.sysroots_dir.join(tag)
    }

    /// The tag the default sysroot symlink resolves to, if it exists
    pub fn current_tag(&self) -> Option<String> {
        let target = std::fs::read_link(&self.sysroot).ok()?;
        let target = if target.is_absolute() {
            target
        } else {
            self.sysroot.parent()?.join(target)
        };
        if !target.is_dir() {
            return None;
        }
        Some(target.file_name()?.to_string_lossy().into_owned())
    }

    /// Points `$NIXWIN_SYSROOT` at the sysroot with the given tag
    pub fn set_default(&self, tag: &str) -> Result<()> {
        let tag_dir = self.tag_dir(tag);
        anyhow::ensure!(
            tag_dir.is_dir(),
            "sysroot '{tag}' is not installed (expected {})",
            tag_dir.display()
        );

        // Use a relative link when the sysroot lives in the default location
        let relative = self.sysroot.parent() == Some(self.data_dir.as_path())
            && self.sysroots_dir == self.data_dir.join("sysroots");
        let target: PathBuf = if relative {
            PathBuf::from(format!("sysroots/{tag}"))
        } else {
            tag_dir
        };

        if self.sysroot.symlink_metadata().is_ok() {
            std::fs::remove_file(&self.sysroot).with_context(|| {
                format!("unable to remove existing sysroot link {}", self.sysroot.display())
            })?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &self.sysroot).with_context(|| {
            format!(
                "unable to symlink {} -> {}",
                self.sysroot.display(),
                target.display()
            )
        })?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, &self.sysroot).with_context(|| {
            format!(
                "unable to symlink {} -> {}",
                self.sysroot.display(),
                target.display()
            )
        })?;

        Ok(())
    }

    /// The machine level configuration file (`config.json`, as opposed to the
    /// per-tag/lockfile `nixwin.json`)
    pub fn machine_config(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }
}

/// Creates a symlink, replacing an existing one at the same location
pub fn symlink_replace(target: &Path, link: &Path) -> Result<()> {
    if let Ok(meta) = link.symlink_metadata() {
        if meta.is_dir() && !meta.is_symlink() {
            anyhow::bail!("refusing to replace directory with symlink: {}", link.display());
        }
        std::fs::remove_file(link)
            .with_context(|| format!("unable to remove existing {}", link.display()))?;
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link)
        .with_context(|| format!("unable to symlink {} -> {}", link.display(), target.display()))?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(target, link)
        .with_context(|| format!("unable to symlink {} -> {}", link.display(), target.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env(home: &Path, vars: Vec<(String, String)>) -> impl Fn(&str) -> Option<String> {
        let vars: std::collections::HashMap<String, String> = std::iter::once((
            "HOME".to_owned(),
            home.display().to_string(),
        ))
        .chain(vars)
        .collect();
        move |key| vars.get(key).cloned()
    }

    #[test]
    fn defaults() {
        let home = tempfile::tempdir().unwrap();
        let env = test_env(home.path(), Vec::new());
        let paths = Paths::resolve_with(None, None, &env).unwrap();

        assert_eq!(paths.data_dir, home.path().join(".local/share/nixwin"));
        assert_eq!(paths.cache_dir, home.path().join(".cache/nixwin"));
        assert_eq!(paths.sysroots_dir, paths.data_dir.join("sysroots"));
        assert_eq!(paths.sysroot, paths.data_dir.join("sysroot"));
        assert_eq!(paths.cache_msvc, paths.cache_dir.join("VC/Tools/MSVC"));
        assert_eq!(
            paths.cache_sdk,
            paths.cache_dir.join("Windows Kits/10")
        );
        assert_eq!(paths.cache_vcr, paths.cache_dir.join("VCR"));
    }

    #[test]
    fn env_overrides() {
        let home = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let sysroots = data.path().join("roots");
        let cache_msvc = data.path().join("msvc");
        let cache_sdk = data.path().join("sdk");
        let cache_vcr = data.path().join("vcr");
        let env = test_env(
            home.path(),
            vec![
                ("NIXWIN_DATA".into(), data.path().to_str().unwrap().into()),
                ("NIXWIN_CACHE".into(), cache.path().to_str().unwrap().into()),
                ("NIXWIN_SYSROOTS".into(), sysroots.to_str().unwrap().into()),
                ("NIXWIN_CACHE_MSVC".into(), cache_msvc.to_str().unwrap().into()),
                ("NIXWIN_CACHE_SDK".into(), cache_sdk.to_str().unwrap().into()),
                ("NIXWIN_CACHE_VCR".into(), cache_vcr.to_str().unwrap().into()),
            ],
        );
        let paths = Paths::resolve_with(None, None, &env).unwrap();

        assert_eq!(paths.data_dir, data.path());
        assert_eq!(paths.cache_dir, cache.path());
        assert_eq!(paths.sysroots_dir, sysroots);
        assert_eq!(paths.sysroot, data.path().join("sysroot"));
        assert_eq!(paths.cache_msvc, cache_msvc);
        assert_eq!(paths.cache_sdk, cache_sdk);
        assert_eq!(paths.cache_vcr, cache_vcr);
    }

    #[test]
    fn flag_beats_env() {
        let home = tempfile::tempdir().unwrap();
        let env = test_env(home.path(), vec![("NIXWIN_DATA".into(), "/from/env".into())]);
        let paths = Paths::resolve_with(Some(PathBuf::from("/from/flag")), None, &env).unwrap();
        assert_eq!(paths.data_dir, PathBuf::from("/from/flag"));
    }

    #[test]
    fn default_link_roundtrip() {
        let home = tempfile::tempdir().unwrap();
        let env = test_env(home.path(), Vec::new());
        let paths = Paths::resolve_with(None, None, &env).unwrap();
        paths.create_dirs().unwrap();

        assert!(paths.current_tag().is_none());

        std::fs::create_dir_all(paths.tag_dir("17")).unwrap();
        std::fs::write(paths.tag_dir("17").join("x"), "x").unwrap();
        paths.set_default("17").unwrap();

        assert_eq!(paths.current_tag().as_deref(), Some("17"));

        // relative link per the README layout
        let link_target = std::fs::read_link(&paths.sysroot).unwrap();
        assert_eq!(link_target, PathBuf::from("sysroots/17"));

        // repoint
        std::fs::create_dir_all(paths.tag_dir("16")).unwrap();
        paths.set_default("16").unwrap();
        assert_eq!(paths.current_tag().as_deref(), Some("16"));

        // dangling link -> no current tag
        std::fs::remove_dir_all(paths.tag_dir("16")).unwrap();
        assert!(paths.current_tag().is_none());
    }
}
