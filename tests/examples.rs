//! End-to-end smoke test for the examples under examples/.
//!
//! Each example builds a windows executable against the nixwin sysroot and
//! prints a well-known marker string when run (under wine when available).
//!
//! Run with: `cargo test --test examples -- --ignored --nocapture`
//!
//! Requires an installed sysroot (e.g. `nixwin install 17 --default`) and
//! build tools: clang-cl/lld-link for the llvm and cmake examples, and cargo
//! with the x86_64-pc-windows-msvc target for the rustc example.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sysroot() -> Option<PathBuf> {
    let root = repo_root();
    let data = std::env::var("NIXWIN_DATA").unwrap_or_else(|_| format!("{}/.nixwin", root.display()));
    let sysroot = std::env::var("NIXWIN_SYSROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(format!("{}/sysroot", data)));
    sysroot.exists().then_some(sysroot)
}

fn build_example(dir: &str) {
    let exe = sysroot().expect("no sysroot installed; run `nixwin install 17 --default` first");
    let cwd = repo_root().join("examples").join(dir);
    let out = Command::new("bash")
        .arg("build.sh")
        .current_dir(&cwd)
        .env("NIXWIN_SYSROOT", &exe)
        .output()
        .expect("failed to spawn bash build.sh");
    assert!(
        out.status.success(),
        "build.sh failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
#[ignore = "requires a nixwin sysroot and windows cross-compile tools"]
fn llvm_example_builds() {
    build_example("llvm");
}

#[test]
#[ignore = "requires a nixwin sysroot and windows cross-compile tools"]
fn cmake_example_builds() {
    build_example("cmake");
}

#[test]
#[ignore = "requires a nixwin sysroot, rustc and the msvc target"]
fn rustc_example_builds() {
    build_example("rustc");
}