//! End-to-end coverage for the managed shell rc block written by `nixwin setup`.
//!
//! Runs the built binary against a temporary `$HOME` so the rc file, the data
//! and the cache directory are all disposable.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PATH_START: &str = "# >>> nixwin >>>";
const PATH_END: &str = "# <<< nixwin <<<";
const CMAKE_START: &str = "# >>> nixwin (cmake) >>>";

struct Home {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_owned();
        Self { _dir: dir, path }
    }

    fn rc(&self, shell: &str) -> PathBuf {
        self.path.join(if shell.ends_with("zsh") {
            ".zshrc"
        } else {
            ".bashrc"
        })
    }

    fn rc_contents(&self, shell: &str) -> String {
        std::fs::read_to_string(self.rc(shell)).unwrap_or_default()
    }

    fn data(&self) -> PathBuf {
        self.path.join("data")
    }

    fn cache(&self) -> PathBuf {
        self.path.join("cache")
    }
}

/// Runs `nixwin setup` against the temporary home, inheriting nothing
fn setup(home: &Home, shell: &str, data: &Path, cache: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nixwin"))
        .arg("setup")
        .args(args)
        .arg("--data-dir")
        .arg(data)
        .arg("--cache-dir")
        .arg(cache)
        .env("HOME", &home.path)
        .env("SHELL", shell)
        .env_remove("NIXWIN_DATA")
        .env_remove("NIXWIN_CACHE")
        .env_remove("NIXWIN_SYSROOT")
        .env_remove("NIXWIN_SYSROOTS")
        .output()
        .expect("failed to run nixwin setup")
}

fn setup_defaults(home: &Home, args: &[&str]) -> Output {
    let (data, cache) = (home.data(), home.cache());
    setup(home, "/bin/bash", &data, &cache, args)
}

fn stdout(out: &Output) -> String {
    assert!(
        out.status.success(),
        "setup failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn assert_exports(contents: &str, data: &Path, cache: &Path) {
    for (name, value) in [
        ("NIXWIN_SYSROOT", data.join("sysroot")),
        ("NIXWIN_DATA", data.to_owned()),
        ("NIXWIN_CACHE", cache.to_owned()),
    ] {
        assert!(
            contents.contains(&format!("export {name}=\"{}\"", value.display())),
            "missing export for {name} in:\n{contents}"
        );
    }
}

#[test]
fn setup_exports_the_paths_without_any_flag() {
    let home = Home::new();
    let out = stdout(&setup_defaults(&home, &[]));
    let contents = home.rc_contents("/bin/bash");

    assert_exports(&contents, &home.data(), &home.cache());
    // no integration flag, so no cmake export
    assert!(!contents.contains("CMAKE_TOOLCHAIN_FILE"));
    assert!(out.contains("added NIXWIN_SYSROOT"));
}

#[test]
fn setup_exports_the_requested_data_and_cache_dirs() {
    let home = Home::new();
    let (data, cache) = (
        home.path.join("custom data"),
        home.path.join("custom cache"),
    );

    setup_defaults(&home, &[]);
    let out = stdout(&setup(&home, "/bin/bash", &data, &cache, &[]));

    let contents = home.rc_contents("/bin/bash");
    assert_exports(&contents, &data, &cache);
    assert!(!contents.contains(&home.data().display().to_string()));
    assert!(out.contains("updated NIXWIN_SYSROOT"));
}

#[test]
fn repeated_setup_keeps_a_single_block() {
    let home = Home::new();
    for _ in 0..3 {
        setup_defaults(&home, &[]);
    }

    let contents = home.rc_contents("/bin/bash");
    assert_eq!(contents.matches(PATH_START).count(), 1);
    assert_eq!(contents.matches(PATH_END).count(), 1);
    assert_eq!(
        contents
            .lines()
            .filter(|l| l.starts_with("export NIXWIN_"))
            .count(),
        3
    );
}

#[test]
fn a_changed_data_dir_is_refreshed() {
    let home = Home::new();
    let moved = home.path.join("moved");

    setup_defaults(&home, &[]);
    setup(&home, "/bin/bash", &moved, &home.cache(), &[]);

    let contents = home.rc_contents("/bin/bash");
    assert_exports(&contents, &moved, &home.cache());
    assert!(!contents.contains(&home.data().display().to_string()));
}

#[test]
fn the_cmake_export_stays_its_own_block() {
    let home = Home::new();

    stdout(&setup_defaults(&home, &["--cmake"]));
    let after_cmake = home.rc_contents("/bin/bash");
    assert_exports(&after_cmake, &home.data(), &home.cache());
    assert!(after_cmake.contains("export CMAKE_TOOLCHAIN_FILE="));
    assert_eq!(after_cmake.matches(CMAKE_START).count(), 1);
    assert_eq!(after_cmake.matches(PATH_START).count(), 1);

    // re-running without --cmake refreshes the path block, leaves the other alone
    let out = stdout(&setup_defaults(&home, &[]));
    let after = home.rc_contents("/bin/bash");
    assert_eq!(after.matches(CMAKE_START).count(), 1);
    assert_eq!(after.matches(PATH_START).count(), 1);
    assert!(after.contains("export CMAKE_TOOLCHAIN_FILE="));
    assert!(out.contains("updated NIXWIN_SYSROOT"));
}

#[test]
fn the_block_lands_in_the_selected_shells_rc() {
    let home = Home::new();
    let (data, cache) = (home.data(), home.cache());

    stdout(&setup(&home, "/bin/zsh", &data, &cache, &[]));
    assert!(home.rc_contents("/bin/zsh").contains("export NIXWIN_DATA="));
    assert!(home.rc_contents("/bin/bash").is_empty());
}

#[test]
fn setup_reports_the_exports_when_no_rc_is_detected() {
    let home = Home::new();
    let (data, cache) = (home.data(), home.cache());

    // without $SHELL there is no rc file to detect
    let out = stdout(
        &Command::new(env!("CARGO_BIN_EXE_nixwin"))
            .arg("setup")
            .arg("--data-dir")
            .arg(&data)
            .arg("--cache-dir")
            .arg(&cache)
            .env("HOME", &home.path)
            .env_remove("SHELL")
            .env_remove("NIXWIN_DATA")
            .env_remove("NIXWIN_CACHE")
            .env_remove("NIXWIN_SYSROOT")
            .env_remove("NIXWIN_SYSROOTS")
            .output()
            .expect("failed to run nixwin setup"),
    );

    let printed = out
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("export "))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        printed,
        format!(
            "export NIXWIN_SYSROOT=\"{}\"\nexport NIXWIN_DATA=\"{}\"\nexport NIXWIN_CACHE=\"{}\"",
            data.join("sysroot").display(),
            data.display(),
            cache.display()
        )
    );
    assert!(out.contains("could not detect your shell rc"));
    assert!(!home.rc("/bin/bash").exists());
}
