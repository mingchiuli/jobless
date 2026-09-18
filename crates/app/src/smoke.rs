use std::thread;
use std::time::Duration;

use jobless_application::RuntimeService;
use jobless_config::LoadedConfig;

pub fn run(loaded: LoadedConfig) -> anyhow::Result<()> {
    let mut runtime = RuntimeService::from_config(&loaded.value, &loaded.paths)?;
    runtime.start()?;
    let health = runtime
        .health()
        .ok_or_else(|| anyhow::anyhow!("runtime health is unavailable"))?;
    tracing::info!(
        runtime = %health.runtime.name,
        runtime_version = %health.runtime.version,
        node_compat_version = %health.runtime.node_compat_version,
        engine = %health.engine.name,
        engine_version = %health.engine.version,
        browser_available = health.browser.available,
        browser_version = health.browser.version.as_deref().unwrap_or("unknown"),
        "browser worker health"
    );

    if !health.browser.available {
        anyhow::bail!("the bundled Patchright browser is not installed");
    }

    runtime.open("about:blank", "smoke")?;
    thread::sleep(Duration::from_millis(500));
    runtime.close()?;
    runtime.stop()?;
    Ok(())
}
