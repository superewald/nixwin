use crate::cmd::install::resolve_tag;
use crate::config::{self, MachineConfig, SysrootConfig};
use crate::{Ctx, emit};
use anyhow::{Context as _, Result};

/// List the installed windows sysroots
pub fn ls(ctx: &Ctx) -> Result<()> {
    let mut tags: Vec<String> = std::fs::read_dir(&ctx.paths.sysroots_dir)
        .with_context(|| {
            format!(
                "unable to read {}",
                ctx.paths.sysroots_dir.display()
            )
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.path().is_dir().then(|| entry.file_name().to_string_lossy().into_owned())
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
pub fn rm(tag: Option<&str>, ctx: &Ctx) -> Result<()> {
    let tag = resolve_tag(tag, ctx)?;
    let tag_dir = ctx.paths.tag_dir(&tag);
    anyhow::ensure!(
        tag_dir.is_dir(),
        "sysroot '{tag}' is not installed (expected {})",
        tag_dir.display()
    );

    std::fs::remove_dir_all(&tag_dir)
        .with_context(|| format!("unable to remove {}", tag_dir.display()))?;

    // repair the default sysroot link if it pointed at the removed tag
    if ctx.paths.current_tag().as_deref() == Some(tag.as_str()) {
        std::fs::remove_file(&ctx.paths.sysroot).with_context(|| {
            format!(
                "unable to remove default sysroot link {}",
                ctx.paths.sysroot.display()
            )
        })?;
    }

    let machine_path = ctx.paths.machine_config();
    if let Ok(Some(mut machine)) = config::load_json::<MachineConfig>(&machine_path)
        && machine.default.tag.as_deref() == Some(tag.as_str())
    {
        machine.default.tag = None;
        config::save_json(&machine_path, &machine)?;
    }

    println!("removed sysroot '{tag}'");
    Ok(())
}

/// Print details about a windows sysroot
pub fn inspect(tag: Option<&str>, ctx: &Ctx) -> Result<()> {
    let tag = resolve_tag(tag, ctx)?;
    let cfg: SysrootConfig = config::load_json(&ctx.paths.tag_dir(&tag).join("nixwin.json"))?
        .with_context(|| format!("sysroot '{tag}' has no nixwin.json, is it installed?"))?;
    println!("{}", emit::inspect_output(&cfg));
    Ok(())
}