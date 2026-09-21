use std::path::{Path, PathBuf};

use jobless_config::{AppConfig, AppPaths};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub runtime_dir: PathBuf,
    pub runtime_bin: PathBuf,
    pub worker_entry: PathBuf,
    pub chromium_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum RuntimePathError {
    #[error("Bun runtime was not found at `{path}`")]
    MissingRuntime { path: PathBuf },
    #[error("browser worker entry was not found at `{path}`")]
    MissingWorker { path: PathBuf },
}

pub fn resolve_runtime_paths(
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<RuntimePaths, RuntimePathError> {
    let runtime_dir = config
        .runtime
        .dir
        .clone()
        .unwrap_or_else(|| paths.runtime_dir.clone());

    let runtime_bin = config
        .runtime
        .bin
        .clone()
        .or_else(|| first_existing(&runtime_candidates(&runtime_dir)))
        .unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "bun.exe" } else { "bun" }));

    let worker_entry = config
        .runtime
        .worker_entry
        .clone()
        .or_else(|| first_existing(&worker_candidates(&runtime_dir)))
        .ok_or_else(|| RuntimePathError::MissingWorker {
            path: runtime_dir.join("browser-worker/src/main.ts"),
        })?;

    if runtime_bin.is_absolute() && !runtime_bin.is_file() {
        return Err(RuntimePathError::MissingRuntime { path: runtime_bin });
    }

    Ok(RuntimePaths {
        runtime_dir: runtime_dir.clone(),
        runtime_bin,
        worker_entry,
        chromium_dir: runtime_dir.join("chromium"),
    })
}

fn worker_candidates(runtime_dir: &Path) -> Vec<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    vec![
        runtime_dir.join("browser-worker/src/main.ts"),
        cwd.join("runtime/browser-worker/src/main.ts"),
        cwd.join("browser-worker/src/main.ts"),
    ]
}

fn runtime_candidates(runtime_dir: &Path) -> Vec<PathBuf> {
    vec![runtime_dir.join("bun/bun.exe"), runtime_dir.join("bun/bun")]
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_development_runtime_from_repo_layout() {
        let root = tempfile::tempdir().unwrap();
        let runtime_dir = root.path().join("runtime");
        let worker = runtime_dir.join("browser-worker/src/main.ts");
        std::fs::create_dir_all(worker.parent().unwrap()).unwrap();
        std::fs::write(&worker, "// worker").unwrap();
        let bun = runtime_dir.join("bun/bun");
        std::fs::create_dir_all(bun.parent().unwrap()).unwrap();
        std::fs::write(&bun, "// bun").unwrap();

        let paths = AppPaths::with_overrides(
            Some(root.path().join("config/config.toml")),
            Some(root.path().join("data")),
            Some(runtime_dir.clone()),
        )
        .unwrap();
        let resolved = resolve_runtime_paths(&AppConfig::default(), &paths).unwrap();

        assert_eq!(resolved.runtime_bin, bun);
        assert_eq!(resolved.worker_entry, worker);
    }
}
