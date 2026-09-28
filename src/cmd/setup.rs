use crate::Ctx;
use crate::config::{self, MachineConfig, SysrootConfig};
use crate::emit;
use crate::paths::Paths;
use anyhow::{Context as _, Result};
use std::path::Path;

#[derive(Debug, Clone, clap::Args)]
pub struct SetupOptions {
    /// Configure the CMake integration: write a toolchain wrapper which
    /// auto-selects the sysroot toolchain when a `.nixwin.json` is present,
    /// and persist `CMAKE_TOOLCHAIN_FILE` in your shell rc
    #[arg(long)]
    pub cmake: bool,
    /// Add `WINEPATH` to your shell rc, so wine resolves the VCR debug
    /// libraries from the default sysroot
    #[arg(long)]
    pub wine: bool,
}

/// Initialize nixwin on the machine, create configuration and tool integration
pub fn setup(opts: &SetupOptions, ctx: &Ctx) -> Result<()> {
    let config_path = ctx.paths.machine_config();
    let mut machine: MachineConfig = config::load_json(&config_path)?.unwrap_or_default();

    setup_paths(shell_rc().as_deref(), &ctx.paths)?;

    if opts.cmake {
        machine.cmake = true;
        setup_cmake(ctx, &machine.tpl)?;
    }

    if opts.wine {
        machine.wine = true;
        setup_wine(ctx)?;
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

const PATH_RC_START: &str = "# >>> nixwin >>>";
const PATH_RC_END: &str = "# <<< nixwin <<<";
const CMAKE_RC_START: &str = "# >>> nixwin (cmake) >>>";
const CMAKE_RC_END: &str = "# <<< nixwin (cmake) <<<";
const WINE_RC_START: &str = "# >>> nixwin (wine) >>>";
const WINE_RC_END: &str = "# <<< nixwin (wine) <<<";

/// What writing a managed block did to the shell rc
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockUpdate {
    /// No block was present, it was appended
    Appended,
    /// A previously written block was replaced
    Replaced,
}

/// The `export` lines making the resolved nixwin paths visible in the shell
fn path_exports(paths: &Paths) -> String {
    [
        ("NIXWIN_SYSROOT", &paths.sysroot),
        ("NIXWIN_DATA", &paths.data_dir),
        ("NIXWIN_CACHE", &paths.cache_dir),
    ]
    .iter()
    .map(|(name, path)| format!("export {name}=\"{}\"", path.display()))
    .collect::<Vec<_>>()
    .join("\n")
}

/// Renders `body` into `contents` between the marker pair, replacing a block
/// written by a previous run. Appends the block when neither marker is present.
fn render_rc_block(
    contents: &str,
    start: &str,
    end: &str,
    body: &str,
) -> Result<(String, BlockUpdate)> {
    let block = format!("{start}\n{body}\n{end}");
    let from = contents.find(start);
    let to = contents.find(end);

    match (from, to) {
        (Some(from), Some(to)) if from < to => {
            // everything up to the end of the end marker line is ours
            let tail = &contents[to + end.len()..];
            let rest = tail.strip_prefix('\n').unwrap_or(tail);
            let mut updated = String::with_capacity(contents.len() + block.len());
            updated.push_str(&contents[..from]);
            updated.push_str(&block);
            updated.push('\n');
            updated.push_str(rest);
            Ok((updated, BlockUpdate::Replaced))
        }
        (None, None) => {
            let mut updated = contents.to_owned();
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(&block);
            updated.push('\n');
            Ok((updated, BlockUpdate::Appended))
        }
        // a lone marker is a block the user edited apart, replacing it would
        // eat unrelated lines and appending would duplicate the block
        _ => anyhow::bail!(
            "unmatched nixwin block marker ('{start}' / '{end}'), fix or remove the nixwin block manually"
        ),
    }
}

/// Writes the managed block to the shell rc, creating it when it does not exist
fn write_rc_block(rc: &Path, start: &str, end: &str, body: &str) -> Result<BlockUpdate> {
    let contents = match std::fs::read_to_string(rc) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(err).with_context(|| format!("unable to read {}", rc.display()));
        }
    };

    let (updated, update) = render_rc_block(&contents, start, end, body)
        .with_context(|| format!("in {}", rc.display()))?;
    std::fs::write(rc, updated).with_context(|| format!("unable to write {}", rc.display()))?;
    Ok(update)
}

/// Exports the resolved nixwin paths to the shell, or prints them when no rc
/// file can be determined
fn setup_paths(rc: Option<&Path>, paths: &Paths) -> Result<()> {
    let body = path_exports(paths);

    let Some(rc) = rc else {
        let manual = body
            .lines()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        println!("could not detect your shell rc, add to your profile:\n{manual}");
        return Ok(());
    };

    match write_rc_block(rc, PATH_RC_START, PATH_RC_END, &body)? {
        BlockUpdate::Replaced => println!(
            "shell rc: {} (updated NIXWIN_SYSROOT, NIXWIN_DATA and NIXWIN_CACHE exports)",
            rc.display()
        ),
        BlockUpdate::Appended => println!(
            "shell rc: {} (added NIXWIN_SYSROOT, NIXWIN_DATA and NIXWIN_CACHE exports)",
            rc.display()
        ),
    }

    Ok(())
}

fn setup_cmake(
    ctx: &Ctx,
    tpl_overrides: &std::collections::BTreeMap<String, std::path::PathBuf>,
) -> Result<()> {
    let wrapper = ctx.paths.data_dir.join("toolchain.cmake");
    std::fs::write(&wrapper, emit::cmake_wrapper(tpl_overrides)?)
        .with_context(|| format!("unable to write {}", wrapper.display()))?;
    println!("cmake wrapper: {}", wrapper.display());

    match shell_rc() {
        Some(rc) => {
            let export = format!("export CMAKE_TOOLCHAIN_FILE=\"{}\"", wrapper.display());
            match write_rc_block(&rc, CMAKE_RC_START, CMAKE_RC_END, &export)? {
                BlockUpdate::Replaced => println!(
                    "shell rc: {} (updated CMAKE_TOOLCHAIN_FILE export)",
                    rc.display()
                ),
                BlockUpdate::Appended => println!(
                    "shell rc: {} (added CMAKE_TOOLCHAIN_FILE export)",
                    rc.display()
                ),
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

/// The directories wine searches for the sysroot's VCR debug libraries, one
/// per configured architecture. Resolved through the default sysroot rather
/// than the cache, so the value follows whichever sysroot is the default.
fn winepath(paths: &Paths, cfg: &SysrootConfig, vcr: &str) -> String {
    cfg.archs
        .iter()
        .map(|arch| {
            paths
                .sysroot
                .join("VCR")
                .join(vcr)
                .join("bin")
                .join(arch.as_str())
        })
        // wine separates WINEPATH entries with `;` on every platform
        .map(|dir| format!("{};", dir.display()))
        .collect::<String>()
        .trim_end_matches(';')
        .to_owned()
}

fn setup_wine(ctx: &Ctx) -> Result<()> {
    let tag = crate::cmd::resolve_tag(None, ctx)?;
    let cfg: SysrootConfig = config::load_json(&ctx.paths.lockfile(&tag))?
        .with_context(|| format!("sysroot '{tag}' has no .nixwin.json, is it installed?"))?;

    let vcr = cfg.vcr.as_ref().context(
        "the default sysroot has no VCR debug libraries, install it with --features debug",
    )?;

    let export = format!("export WINEPATH=\"{}\"", winepath(&ctx.paths, &cfg, vcr));

    match shell_rc() {
        Some(rc) => match write_rc_block(&rc, WINE_RC_START, WINE_RC_END, &export)? {
            BlockUpdate::Replaced => {
                println!("shell rc: {} (updated WINEPATH export)", rc.display())
            }
            BlockUpdate::Appended => {
                println!("shell rc: {} (added WINEPATH export)", rc.display())
            }
        },
        None => {
            println!("could not detect your shell rc, add to your profile:\n  {export}");
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::{Arch, Feature};
    use std::path::PathBuf;

    /// `Paths` for a fixed home, without reading the real environment
    fn test_paths(data: PathBuf, cache: PathBuf) -> Paths {
        let home = data.parent().expect("data dir needs a parent").to_owned();
        let env = |key: &str| (key == "HOME").then(|| home.display().to_string());
        Paths::resolve_with(Some(data), Some(cache), &env).unwrap()
    }

    fn temp_paths() -> (tempfile::TempDir, Paths) {
        let dir = tempfile::tempdir().unwrap();
        let paths = test_paths(dir.path().join("data"), dir.path().join("cache"));
        (dir, paths)
    }

    #[test]
    fn block_is_appended_to_a_missing_rc() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");

        let update = write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"1\"").unwrap();
        assert_eq!(update, BlockUpdate::Appended);

        let contents = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(
            contents,
            format!("{PATH_RC_START}\nexport A=\"1\"\n{PATH_RC_END}\n")
        );
    }

    #[test]
    fn block_is_appended_after_an_unterminated_line() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(&rc, "export MINE=1").unwrap();

        let update = write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"1\"").unwrap();
        assert_eq!(update, BlockUpdate::Appended);

        // the last line before the block stays a line of its own
        let contents = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(
            contents,
            format!("export MINE=1\n{PATH_RC_START}\nexport A=\"1\"\n{PATH_RC_END}\n")
        );
    }

    #[test]
    fn block_is_rewritten_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(&rc, "before\nafter\n").unwrap();

        write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"old\"").unwrap();
        let update = write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"new\"").unwrap();
        assert_eq!(update, BlockUpdate::Replaced);

        let contents = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(
            contents,
            format!("before\nafter\n{PATH_RC_START}\nexport A=\"new\"\n{PATH_RC_END}\n")
        );
        assert!(!contents.contains("old"));
        assert_eq!(contents.matches(PATH_RC_START).count(), 1);
        assert_eq!(contents.matches(PATH_RC_END).count(), 1);
    }

    #[test]
    fn three_writes_leave_one_block() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");

        let mut line_counts = Vec::new();
        for value in ["a", "b", "c"] {
            let body = format!("export A=\"{value}\"\nexport B=\"{value}\"\nexport C=\"{value}\"");
            write_rc_block(&rc, PATH_RC_START, PATH_RC_END, &body).unwrap();
            let contents = std::fs::read_to_string(&rc).unwrap();
            // rewriting does not accumulate newlines or blank lines
            line_counts.push(contents.lines().count());
        }
        assert!(
            line_counts.windows(2).all(|w| w[0] == w[1]),
            "rewriting changed the line count: {line_counts:?}"
        );

        let contents = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(contents.matches(PATH_RC_START).count(), 1);
        assert_eq!(contents.matches(PATH_RC_END).count(), 1);
        assert!(contents.contains("export A=\"c\""));
        assert!(!contents.contains("export A=\"a\""));
        assert!(!contents.contains("export A=\"b\""));
        // three exports
        assert_eq!(
            contents
                .lines()
                .filter(|l| l.starts_with("export "))
                .count(),
            3
        );
    }

    #[test]
    fn rewriting_keeps_a_mid_file_block_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        std::fs::write(
            &rc,
            format!("before\n{PATH_RC_START}\nexport A=\"old\"\n{PATH_RC_END}\nafter\n"),
        )
        .unwrap();

        write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"new\"").unwrap();
        let contents = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(
            contents,
            format!("before\n{PATH_RC_START}\nexport A=\"new\"\n{PATH_RC_END}\nafter\n")
        );

        write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"new\"").unwrap();
        assert_eq!(std::fs::read_to_string(&rc).unwrap(), contents);
    }

    #[test]
    fn unmatched_marker_is_reported_and_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        let broken = format!("{PATH_RC_START}\nexport A=\"1\"\n");
        std::fs::write(&rc, &broken).unwrap();

        let err = write_rc_block(&rc, PATH_RC_START, PATH_RC_END, "export A=\"2\"").unwrap_err();
        assert!(format!("{err:#}").contains("unmatched nixwin block marker"));
        assert_eq!(std::fs::read_to_string(&rc).unwrap(), broken);
    }

    #[test]
    fn exports_carry_the_resolved_paths() {
        let (_dir, paths) = temp_paths();
        let body = path_exports(&paths);

        assert_eq!(
            body,
            format!(
                "export NIXWIN_SYSROOT=\"{}\"\nexport NIXWIN_DATA=\"{}\"\nexport NIXWIN_CACHE=\"{}\"",
                paths.sysroot.display(),
                paths.data_dir.display(),
                paths.cache_dir.display()
            )
        );
    }

    #[test]
    fn exports_follow_the_data_dir() {
        let (_dir, paths) = temp_paths();
        let body = path_exports(&paths);

        assert!(body.contains(&format!(
            "export NIXWIN_DATA=\"{}\"",
            paths.data_dir.display()
        )));
        // the sysroot follows the data dir instead of the built-in default
        assert_eq!(paths.sysroot, paths.data_dir.join("sysroot"));
        assert!(body.contains(&format!(
            "export NIXWIN_SYSROOT=\"{}\"",
            paths.data_dir.join("sysroot").display()
        )));
    }

    #[test]
    fn exports_follow_the_cache_dir() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let cache = dir.path().join("some other cache");
        let body = path_exports(&test_paths(data.clone(), cache.clone()));

        assert!(body.contains(&format!("export NIXWIN_CACHE=\"{}\"", cache.display())));
        // the cache dir does not influence the data side
        assert!(body.contains(&format!("export NIXWIN_DATA=\"{}\"", data.display())));
    }

    #[cfg(unix)]
    #[test]
    fn exports_survive_spaces_in_paths() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("my data dir");
        let rc = dir.path().join("rc file");
        let body = path_exports(&test_paths(data.clone(), dir.path().join("my cache dir")));
        write_rc_block(&rc, PATH_RC_START, PATH_RC_END, &body).unwrap();

        // the block has to be valid shell, not just parse as one
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(r#". "$1"; printf '%s\n%s' "$NIXWIN_DATA" "$NIXWIN_CACHE""#)
            .arg("sh")
            .arg(&rc)
            .output()
            .unwrap();
        assert!(out.status.success(), "block is not valid shell");

        let printed = String::from_utf8(out.stdout).unwrap();
        assert_eq!(
            printed,
            format!(
                "{}\n{}",
                data.display(),
                dir.path().join("my cache dir").display()
            )
        );
    }

    #[test]
    fn setup_paths_refreshes_the_block() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".zshrc");
        let first = test_paths(dir.path().join("data"), dir.path().join("cache"));

        setup_paths(Some(&rc), &first).unwrap();
        setup_paths(Some(&rc), &first).unwrap();
        let second = test_paths(dir.path().join("moved"), dir.path().join("cache"));
        setup_paths(Some(&rc), &second).unwrap();

        let contents = std::fs::read_to_string(&rc).unwrap();
        assert_eq!(contents.matches(PATH_RC_START).count(), 1);
        assert!(contents.contains("export NIXWIN_DATA=\""));
        assert!(!contents.contains(&dir.path().join("data").display().to_string()));
        assert!(contents.contains(&dir.path().join("moved").display().to_string()));
    }

    #[test]
    fn setup_paths_without_an_rc_is_not_fatal() {
        let (_dir, paths) = temp_paths();
        assert!(setup_paths(None, &paths).is_ok());
    }

    #[test]
    fn cmake_and_path_blocks_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let rc = dir.path().join(".bashrc");
        let paths = test_paths(dir.path().join("data"), dir.path().join("cache"));

        setup_paths(Some(&rc), &paths).unwrap();
        write_rc_block(
            &rc,
            CMAKE_RC_START,
            CMAKE_RC_END,
            "export CMAKE_TOOLCHAIN_FILE=\"/a\"",
        )
        .unwrap();
        write_rc_block(
            &rc,
            CMAKE_RC_START,
            CMAKE_RC_END,
            "export CMAKE_TOOLCHAIN_FILE=\"/b\"",
        )
        .unwrap();
        // refreshing the path block must not disturb the cmake block
        setup_paths(Some(&rc), &paths).unwrap();

        let contents = std::fs::read_to_string(&rc).unwrap();
        assert!(contents.contains("export NIXWIN_DATA="));
        assert!(contents.contains("export CMAKE_TOOLCHAIN_FILE=\"/b\""));
        assert!(!contents.contains("\"/a\""));
        assert_eq!(contents.matches(CMAKE_RC_START).count(), 1);
        assert_eq!(contents.matches(PATH_RC_START).count(), 1);
    }

    fn sysroot_cfg(archs: Vec<Arch>, vcr: Option<&str>) -> SysrootConfig {
        SysrootConfig {
            tag: "17".into(),
            manifest: 17,
            channel: "release".into(),
            archs,
            variants: Vec::new(),
            features: vec![Feature::Debug],
            sdk: "10.0.26100".into(),
            crt: "14.44.17.14".into(),
            vcr: vcr.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn winepath_for_a_single_architecture() {
        let (_dir, paths) = temp_paths();
        let cfg = sysroot_cfg(vec![Arch::X86_64], Some("14.44.35211"));

        assert_eq!(
            winepath(&paths, &cfg, "14.44.35211"),
            paths
                .sysroot
                .join("VCR/14.44.35211/bin/x86_64")
                .display()
                .to_string()
        );
    }

    #[test]
    fn winepath_for_a_mixed_sysroot() {
        let (_dir, paths) = temp_paths();
        let cfg = sysroot_cfg(vec![Arch::X86_64, Arch::X86], Some("14.44.35211"));

        assert_eq!(
            winepath(&paths, &cfg, "14.44.35211"),
            format!(
                "{};{}",
                paths.sysroot.join("VCR/14.44.35211/bin/x86_64").display(),
                paths.sysroot.join("VCR/14.44.35211/bin/x86").display()
            )
        );
    }

    #[test]
    fn winepath_resolves_through_the_sysroot_not_the_cache() {
        let (_dir, paths) = temp_paths();
        let cfg = sysroot_cfg(vec![Arch::X86_64], Some("14.44.35211"));

        let value = winepath(&paths, &cfg, "14.44.35211");
        assert!(value.starts_with(&paths.sysroot.display().to_string()));
        assert!(!value.contains(&paths.cache_dir.display().to_string()));
        // a directory, not a file
        assert!(!value.ends_with(".dll"));
    }

    #[test]
    fn winepath_follows_the_configured_architecture_order() {
        let (_dir, paths) = temp_paths();
        let cfg = sysroot_cfg(vec![Arch::X86, Arch::X86_64], Some("14.44.35211"));

        assert_eq!(
            winepath(&paths, &cfg, "14.44.35211"),
            format!(
                "{};{}",
                paths.sysroot.join("VCR/14.44.35211/bin/x86").display(),
                paths.sysroot.join("VCR/14.44.35211/bin/x86_64").display()
            )
        );
    }
}
