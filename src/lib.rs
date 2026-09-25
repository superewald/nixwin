use crate::paths::Paths;
use anyhow::Context as _;
use indicatif::MultiProgress;
use std::{path::PathBuf, sync::Arc};
use xwin::{Payload, WorkItem, ureq::Agent, util::ProgressTarget};

pub mod cmd;
pub mod config;
pub mod emit;
pub mod paths;

/// The nixwin execution context: resolved directories + xwin plumbing
#[derive(Debug, Clone)]
pub struct Ctx {
    pub paths: Paths,
    /// xwin work directory (download + unpack caches), lives under the cache
    pub work_dir: PathBuf,
}

impl Ctx {
    pub fn new(
        data_dir: Option<PathBuf>,
        cache_dir: Option<PathBuf>,
    ) -> Result<Self, anyhow::Error> {
        let env = paths::real_env();
        Self::with_env(data_dir, cache_dir, &env)
    }

    /// Testable variant with an injectable environment lookup
    pub fn with_env(
        data_dir: Option<PathBuf>,
        cache_dir: Option<PathBuf>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let paths = Paths::resolve_with(data_dir, cache_dir, env)?;
        paths.create_dirs()?;
        let work_dir = paths.cache_dir.join("xwin");
        Ok(Self { paths, work_dir })
    }

    pub fn xwin_ctx(&self) -> Result<Arc<xwin::Ctx>, anyhow::Error> {
        Ok(Arc::new(xwin::Ctx::with_dir(
            self.work_dir
                .to_str()
                .context("cache dir must be valid utf-8")?
                .to_owned()
                .into(),
            ProgressTarget::Stdout,
            Agent::new_with_defaults(),
            5,
        )?))
    }
}

/// Whether nixwin is running in a CI environment
pub fn in_ci() -> bool {
    [
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "CIRCLECI",
        "TRAVIS",
        "BUILDKITE",
        "DRONE",
        "TF_BUILD",
    ]
    .iter()
    .any(|key| {
        std::env::var(key).is_ok_and(|value| {
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        })
    })
}

pub fn payloads_to_work_items(
    payloads: &[Payload],
    mp: MultiProgress,
    draw_target: ProgressTarget,
) -> Vec<WorkItem> {
    let work_items: Vec<_> = payloads
        .iter()
        .map(|pay| {
            use xwin::PayloadKind;

            let prefix = match pay.kind {
                PayloadKind::CrtHeaders => "CRT.headers".to_owned(),
                PayloadKind::AtlHeaders => "ATL.headers".to_owned(),
                PayloadKind::CrtLibs => {
                    format!(
                        "CRT.libs.{}.{}",
                        pay.target_arch.map_or("all", |ta| ta.as_str()),
                        pay.variant.map_or("none", |v| v.as_str())
                    )
                }
                PayloadKind::AtlLibs => {
                    format!(
                        "ATL.libs.{}",
                        pay.target_arch.map_or("all", |ta| ta.as_str()),
                    )
                }
                PayloadKind::SdkHeaders => {
                    format!(
                        "SDK.headers.{}.{}",
                        pay.target_arch.map_or("all", |v| v.as_str()),
                        pay.variant.map_or("none", |v| v.as_str())
                    )
                }
                PayloadKind::SdkLibs => {
                    format!(
                        "SDK.libs.{}",
                        pay.target_arch.map_or("all", |ta| ta.as_str())
                    )
                }
                PayloadKind::SdkStoreLibs => "SDK.libs.store.all".to_owned(),
                PayloadKind::Ucrt => "SDK.ucrt.all".to_owned(),
                PayloadKind::VcrDebug => {
                    let prefix = match pay.filename.to_string().contains("UCRT") {
                        true => "UCRT.Debug",
                        false => "VC.Runtime.Debug",
                    };

                    format!(
                        "{}.{}",
                        prefix,
                        pay.target_arch.map_or("all", |ta| ta.as_str())
                    )
                }
            };

            let pb = mp.add(
                indicatif::ProgressBar::with_draw_target(Some(0), draw_target.into()).with_prefix(prefix).with_style(
                    indicatif::ProgressStyle::default_bar()
                        .template("{spinner:.green} {prefix:.bold} [{elapsed}] {wide_bar:.green} {bytes}/{total_bytes} {msg}")
                        .unwrap()
                        .progress_chars("█▇▆▅▄▃▂▁  "),
                ),
            );
            xwin::WorkItem {
                payload: std::sync::Arc::new(pay.clone()),
                progress: pb,
            }
        })
        .collect();
    work_items
}
