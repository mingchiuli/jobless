use gpui_kit::base::{Disableable, StyledExt, h_flex, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::*;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use rust_i18n::t;

use super::super::{AppView, RuntimeStatus};
use super::page_title;

impl AppView {
    pub(in crate::app) fn browser_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let runtime_ready = self.runtime_ready();
        let browser_open = self.browser_open();

        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_bold().text_xl().child(page_title(self.page)))
                    .child(
                        h_flex()
                            .gap_1()
                            .child(div().text_sm().child(self.status_label()))
                            .child(div().size_2().rounded_full().bg(match self.status {
                                RuntimeStatus::Running => cx.theme().success,
                                RuntimeStatus::Failed => cx.theme().danger,
                                RuntimeStatus::Stopped => cx.theme().muted_foreground,
                                RuntimeStatus::Starting | RuntimeStatus::Stopping => {
                                    cx.theme().warning
                                }
                            })),
                    ),
            )
            .child(
                div()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .child(self.detail_label()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("start-runtime")
                            .primary()
                            .label(t!("browser.start_runtime").to_string())
                            .disabled(runtime_ready)
                            .on_click(cx.listener(|this, _, _, cx| this.start_runtime(cx))),
                    )
                    .child(
                        Button::new("open-browser")
                            .label(t!("browser.open_test_page").to_string())
                            .disabled(!runtime_ready || browser_open)
                            .on_click(cx.listener(|this, _, _, cx| this.open_smoke_page(cx))),
                    )
                    .child(
                        Button::new("close-browser")
                            .label(t!("browser.close_browser").to_string())
                            .disabled(!browser_open)
                            .on_click(cx.listener(|this, _, _, cx| this.close_browser(cx))),
                    )
                    .child(
                        Button::new("stop-runtime")
                            .danger()
                            .label(t!("browser.stop_runtime").to_string())
                            .disabled(!runtime_ready)
                            .on_click(cx.listener(|this, _, _, cx| this.stop_runtime(cx))),
                    ),
            )
            .child(
                div()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .v_flex()
                    .gap_2()
                    .child(div().font_bold().child(t!("browser.paths").to_string()))
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Config: {}", self.paths.config_file.display())),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Data: {}", self.paths.data_dir.display())),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Runtime: {}", self.paths.runtime_dir.display())),
                    ),
            )
            .into_any_element()
    }

    fn status_label(&self) -> String {
        match self.status {
            RuntimeStatus::Stopped => t!("runtime.status.stopped").to_string(),
            RuntimeStatus::Starting => t!("runtime.status.starting").to_string(),
            RuntimeStatus::Running => t!("runtime.status.running").to_string(),
            RuntimeStatus::Stopping => t!("runtime.status.stopping").to_string(),
            RuntimeStatus::Failed => t!("runtime.status.failed").to_string(),
        }
    }

    fn detail_label(&self) -> String {
        match self.detail.as_str() {
            "runtime.idle" => t!("runtime.idle").to_string(),
            "runtime.starting" => t!("runtime.starting").to_string(),
            "runtime.ready" => t!("runtime.ready").to_string(),
            "runtime.stopping" => t!("runtime.stopping").to_string(),
            "browser.opening" => t!("browser.opening").to_string(),
            "browser.opened" => t!("browser.opened").to_string(),
            "browser.closing" => t!("browser.closing").to_string(),
            "browser.closed" => t!("browser.closed").to_string(),
            other => other.to_string(),
        }
    }
}
