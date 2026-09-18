mod components;
mod pages;
mod theme;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme, Root};
use gpui_kit::*;
use jobless_application::RuntimeService;
use jobless_config::{AppConfig, AppPaths, LoadedConfig};
use jobless_storage::Database;
use theme::ThemePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Accounts,
    Tasks,
    Browser,
    History,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

struct AppView {
    page: Page,
    status: RuntimeStatus,
    detail: String,
    config: AppConfig,
    paths: AppPaths,
    storage: Option<Database>,
    runtime: Option<RuntimeService>,
    theme_preference: ThemePreference,
    appearance_subscription: Option<Subscription>,
}

impl AppView {
    fn new(loaded: LoadedConfig) -> Self {
        let (storage, error) = match Database::open_from_paths(&loaded.paths) {
            Ok(database) => (Some(database), None),
            Err(error) => (None, Some(error.to_string())),
        };

        let (runtime, runtime_error) =
            match RuntimeService::from_config(&loaded.value, &loaded.paths) {
                Ok(runtime) => (Some(runtime), None),
                Err(error) => (None, Some(error.to_string())),
            };

        Self {
            page: Page::Browser,
            status: if error.is_some() || runtime_error.is_some() {
                RuntimeStatus::Failed
            } else {
                RuntimeStatus::Stopped
            },
            detail: error
                .or(runtime_error)
                .unwrap_or_else(|| "runtime.idle".to_string()),
            config: loaded.value,
            paths: loaded.paths,
            storage,
            runtime,
            theme_preference: ThemePreference::System,
            appearance_subscription: None,
        }
    }

    fn install_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_preference.apply(Some(window), cx);
        self.appearance_subscription =
            Some(cx.observe_window_appearance(window, |this, window, cx| {
                if this.theme_preference == ThemePreference::System {
                    ThemePreference::System.apply(Some(window), cx);
                    cx.notify();
                }
            }));
    }

    fn cycle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_preference = self.theme_preference.next();
        self.theme_preference.apply(Some(window), cx);
        cx.notify();
    }

    fn runtime_ready(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(RuntimeService::is_running)
    }

    fn browser_open(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(RuntimeService::is_browser_open)
    }

    fn start_runtime(&mut self, cx: &mut Context<Self>) {
        if self.runtime_ready() {
            return;
        }

        let Some(mut runtime) = self.runtime.take() else {
            self.status = RuntimeStatus::Failed;
            self.detail = "runtime.unavailable".to_string();
            cx.notify();
            return;
        };

        self.status = RuntimeStatus::Starting;
        self.detail = "runtime.starting".to_string();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (runtime, result) = cx
                .background_spawn(async move {
                    let result = runtime.start();
                    (runtime, result)
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        this.status = RuntimeStatus::Running;
                        this.detail = "runtime.ready".to_string();
                    }
                    Err(error) => {
                        this.status = RuntimeStatus::Failed;
                        this.detail = error.to_string();
                    }
                }
                this.runtime = Some(runtime);
                cx.notify();
            });
        })
        .detach();
    }

    fn open_smoke_page(&mut self, cx: &mut Context<Self>) {
        let Some(mut runtime) = self.runtime.take() else {
            return;
        };
        let url = self.config.browser.smoke_url.clone();

        self.status = RuntimeStatus::Starting;
        self.detail = "browser.opening".to_string();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (runtime, result) = cx
                .background_spawn(async move {
                    let result = runtime.open(url, "default");
                    (runtime, result)
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.status = RuntimeStatus::Running;
                        this.detail = "browser.opened".to_string();
                    }
                    Err(error) => {
                        this.status = RuntimeStatus::Failed;
                        this.detail = error.to_string();
                    }
                }
                this.runtime = Some(runtime);
                cx.notify();
            });
        })
        .detach();
    }

    fn close_browser(&mut self, cx: &mut Context<Self>) {
        let Some(mut runtime) = self.runtime.take() else {
            return;
        };

        self.detail = "browser.closing".to_string();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (runtime, result) = cx
                .background_spawn(async move {
                    let result = runtime.close();
                    (runtime, result)
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.status = RuntimeStatus::Running;
                        this.detail = "browser.closed".to_string();
                    }
                    Err(error) => {
                        this.status = RuntimeStatus::Failed;
                        this.detail = error.to_string();
                    }
                }
                this.runtime = Some(runtime);
                cx.notify();
            });
        })
        .detach();
    }

    fn stop_runtime(&mut self, cx: &mut Context<Self>) {
        let Some(mut runtime) = self.runtime.take() else {
            return;
        };

        self.status = RuntimeStatus::Stopping;
        self.detail = "runtime.stopping".to_string();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (runtime, result) = cx
                .background_spawn(async move {
                    let result = runtime.stop();
                    (runtime, result)
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(()) => {
                        this.status = RuntimeStatus::Stopped;
                        this.detail = "runtime.idle".to_string();
                    }
                    Err(error) => {
                        this.status = RuntimeStatus::Failed;
                        this.detail = error.to_string();
                    }
                }
                this.runtime = Some(runtime);
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Page::Browser => self.browser_page(cx),
            Page::Settings => self.settings_page(cx),
            Page::Accounts => self.accounts_page(cx),
            Page::Tasks => self.tasks_page(cx),
            Page::History => self.history_page(cx),
        };

        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.sidebar(cx))
            .child(v_flex().h_full().flex_1().min_w_0().p_6().child(content))
    }
}

pub fn run(loaded: LoadedConfig, smoke_test: bool) -> anyhow::Result<()> {
    let locale = loaded.value.app.locale.clone();
    let smoke_loaded = loaded.clone();
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::component::set_locale(&locale);
        let window_bounds = WindowBounds::centered(size(px(1100.), px(720.)), cx);

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(window_bounds),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|_| AppView::new(loaded.clone()));
                    view.update(cx, |this, cx| this.install_theme(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("failed to open Jobless window");

            if smoke_test {
                let result = cx
                    .background_spawn(async move { crate::smoke::run(smoke_loaded) })
                    .await;
                match result {
                    Ok(()) => tracing::info!("SMOKE_OK"),
                    Err(error) => tracing::error!(error = %error, "SMOKE_FAILED"),
                }
                cx.update(|cx| cx.quit());
            }
        })
        .detach();
    });

    Ok(())
}
