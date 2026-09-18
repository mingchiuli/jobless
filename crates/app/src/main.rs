rust_i18n::i18n!("../../locales", fallback = "zh-CN");

mod app;
mod smoke;

use std::path::PathBuf;

use anyhow::Context;
use jobless_config::{AppPaths, ConfigStore};
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    let smoke_test = std::env::args().any(|argument| argument == "--smoke-test");
    let smoke_root = smoke_test
        .then(|| std::env::temp_dir().join(format!("jobless-smoke-{}", std::process::id())));

    let paths = match smoke_root.as_ref() {
        Some(root) => AppPaths::with_overrides(
            Some(root.join("config/config.toml")),
            Some(root.join("data")),
            std::env::var_os("JOBLESS_RUNTIME_DIR").map(PathBuf::from),
        ),
        None => AppPaths::discover(),
    }
    .context("failed to resolve application paths")?;

    let store = ConfigStore::new(paths);
    let loaded = store.load().context("failed to load configuration")?;
    init_tracing(&loaded.value.logging.level);

    tracing::info!(
        config = %loaded.paths.config_file.display(),
        data = %loaded.paths.data_dir.display(),
        smoke_test,
        "starting Jobless"
    );

    let result = app::run(loaded, smoke_test);

    if let Some(root) = smoke_root {
        let _ = std::fs::remove_dir_all(root);
    }

    result
}

fn init_tracing(level: &str) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
