//! End-to-end coverage for the `WINEPATH` export written by `nixwin setup --wine`.
//!
//! Runs the built binary against a temporary `$HOME` with a fabricated
//! sysroot, so no install and no network access is required.
//!
//! The sysroot's `sysroot` entry is a symlink, which the tests need, hence the
//! unix gate.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const WINE_START: &str = "# >>> nixwin (wine) >>>";
const WINE_END: &str = "# <<< nixwin (wine) <<<";

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

    fn data(&self) -> PathBuf {
        self.path.join("data")
    }

    fn cache(&self) -> PathBuf {
        self.path.join("cache")
    }

    fn rc(&self) -> PathBuf {
        self.path.join(".bashrc")
    }

    fn rc_contents(&self) -> String {
        std::fs::read_to_string(self.rc()).unwrap_or_default()
    }

    /// Creates a sysroot and points the `sysroot` symlink at it, which is what
    /// makes it the default one
    fn install_sysroot(&self, tag: &str, vcr: Option<&str>, archs: &[&str]) {
        let tag_dir = self.data().join("sysroots").join(tag);
        std::fs::create_dir_all(&tag_dir).unwrap();

        let archs_json = archs
            .iter()
            .map(|arch| format!("\"{arch}\""))
            .collect::<Vec<_>>()
            .join(",");
        let vcr_field = vcr.map_or(String::new(), |v| format!(r#","vcr":"{v}""#));
        std::fs::write(
            tag_dir.join(".nixwin.json"),
            format!(
                r#"{{"tag":"{tag}","manifest":17,"channel":"release","archs":[{archs_json}],"variants":["desktop"],"features":["debug"],"sdk":"10.0.26100","crt":"14.44.17.14"{vcr_field}}}"#
            ),
        )
        .unwrap();

        for arch in archs {
            let dir = tag_dir
                .join("VCR")
                .join(vcr.unwrap_or("none"))
                .join("bin")
                .join(arch);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("vcruntime140d.dll"), "stub").unwrap();
        }

        self.set_default(tag);
    }

    fn set_default(&self, tag: &str) {
        let link = self.data().join("sysroot");
        let _ = std::fs::remove_file(&link);
        // relative, matching the layout `nixwin install --default` creates
        std::os::unix::fs::symlink(format!("sysroots/{tag}"), &link).unwrap();
    }
}

fn setup(home: &Home, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nixwin"))
        .arg("setup")
        .args(args)
        .arg("--data-dir")
        .arg(home.data())
        .arg("--cache-dir")
        .arg(home.cache())
        .env("HOME", &home.path)
        .env("SHELL", "/bin/bash")
        .env_remove("NIXWIN_DATA")
        .env_remove("NIXWIN_CACHE")
        .env_remove("NIXWIN_SYSROOT")
        .env_remove("NIXWIN_SYSROOTS")
        .env_remove("WINEPREFIX")
        .output()
        .expect("failed to run nixwin setup")
}

fn stdout(out: &Output) -> String {
    assert!(
        out.status.success(),
        "setup failed:\n{}\n{}",
        out_text(&out.stdout),
        out_text(&out.stderr)
    );
    out_text(&out.stdout)
}

fn stderr(out: &Output) -> String {
    out_text(&out.stderr)
}

fn out_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn vcr_dirs(home: &Home, vcr: &str, archs: &[&str]) -> String {
    archs
        .iter()
        .map(|arch| {
            home.data()
                .join("sysroot")
                .join("VCR")
                .join(vcr)
                .join("bin")
                .join(arch)
                .display()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut entries = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let child = entry.path();
            if child.is_dir() {
                stack.push(child);
            } else {
                entries.push((
                    child.strip_prefix(dir).unwrap().to_owned(),
                    std::fs::read(&child).unwrap(),
                ));
            }
        }
    }
    entries.sort();
    entries
}

#[test]
fn setup_wine_exports_the_sysroot_vcr_directories() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64"]);

    let out = stdout(&setup(&home, &["--wine"]));
    let contents = home.rc_contents();

    assert!(
        contents.contains(&format!(
            "export WINEPATH=\"{}\"",
            vcr_dirs(&home, "14.44.35211", &["x86_64"])
        )),
        "unexpected WINEPATH in:\n{contents}"
    );
    assert!(out.contains("added WINEPATH export"));
}

#[test]
fn every_configured_architecture_gets_a_directory() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64", "x86"]);

    stdout(&setup(&home, &["--wine"]));
    let contents = home.rc_contents();

    assert!(contents.contains(&format!(
        "export WINEPATH=\"{}\"",
        vcr_dirs(&home, "14.44.35211", &["x86_64", "x86"])
    )));
}

#[test]
fn repeated_setup_keeps_one_export() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64"]);

    for _ in 0..3 {
        stdout(&setup(&home, &["--wine"]));
    }

    let contents = home.rc_contents();
    assert_eq!(contents.matches(WINE_START).count(), 1);
    assert_eq!(contents.matches(WINE_END).count(), 1);
    assert_eq!(contents.matches("export WINEPATH=").count(), 1);
}

#[test]
fn a_changed_default_sysroot_refreshes_the_export() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64"]);
    stdout(&setup(&home, &["--wine"]));
    assert!(home.rc_contents().contains("14.44.35211"));

    // switch the default to a sysroot built against another VCR version
    home.install_sysroot("18", Some("14.46.1234"), &["x86_64"]);
    home.set_default("18");
    let out = stdout(&setup(&home, &["--wine"]));
    let contents = home.rc_contents();

    assert!(contents.contains(&format!(
        "export WINEPATH=\"{}\"",
        vcr_dirs(&home, "14.46.1234", &["x86_64"])
    )));
    assert!(!contents.contains("14.44.35211"));
    assert!(out.contains("updated WINEPATH export"));
    assert_eq!(contents.matches("export WINEPATH=").count(), 1);
}

#[test]
fn setup_without_wine_writes_no_export() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64"]);

    stdout(&setup(&home, &[]));
    stdout(&setup(&home, &["--cmake"]));

    let contents = home.rc_contents();
    assert!(!contents.contains("WINEPATH"));
    assert!(!contents.contains(WINE_START));
}

#[test]
fn the_wine_prefix_is_left_untouched() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64", "x86"]);

    // a prefix as an earlier nixwin version would have left it
    for dir in ["drive_c/windows/system32", "drive_c/windows/syswow64"] {
        let dir = home.path.join(".wine").join(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vcruntime140d.dll"), "installed by nixwin").unwrap();
    }
    let before = tree(&home.path.join(".wine"));

    stdout(&setup(&home, &["--wine"]));

    assert_eq!(tree(&home.path.join(".wine")), before);
    let out = stdout(&setup(&home, &["--wine"]));
    assert!(!out.contains("dll(s) installed"));
}

#[test]
fn the_wine_prefix_option_is_gone() {
    let home = Home::new();
    home.install_sysroot("17", Some("14.44.35211"), &["x86_64"]);

    let out = setup(&home, &["--wine", "--wine-prefix", "/tmp/prefix"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("--wine-prefix"),
        "unexpected error:\n{}",
        stderr(&out)
    );
    assert!(!home.rc_contents().contains("WINEPATH"));
}

#[test]
fn no_default_sysroot_is_an_error() {
    let home = Home::new();

    let out = setup(&home, &["--wine"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("no default sysroot set"),
        "unexpected error:\n{}",
        stderr(&out)
    );
    // nothing about wine was written, and no export before the failure
    assert!(!home.rc_contents().contains("WINEPATH"));
    assert!(!out_text(&out.stdout).contains("WINEPATH"));
}

#[test]
fn a_sysroot_without_vcr_debug_libraries_is_an_error() {
    let home = Home::new();
    home.install_sysroot("17", None, &["x86_64"]);

    let out = setup(&home, &["--wine"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("no VCR debug libraries"),
        "unexpected error:\n{}",
        stderr(&out)
    );
    // the remedy names the feature
    assert!(stderr(&out).contains("--features debug"));
    assert!(!home.rc_contents().contains("WINEPATH"));
    assert!(!out_text(&out.stdout).contains("WINEPATH"));
}
