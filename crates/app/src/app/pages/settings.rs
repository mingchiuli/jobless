use gpui_kit::base::{StyledExt, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use rust_i18n::t;

use super::super::AppView;
use super::page_title;

impl AppView {
    pub(in crate::app) fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let storage = self
            .storage
            .as_ref()
            .map(|database| database.path().display().to_string())
            .unwrap_or_else(|| t!("settings.storage_unavailable").to_string());

        v_flex()
            .gap_4()
            .child(div().font_bold().text_xl().child(page_title(self.page)))
            .child(
                div()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .p_4()
                    .v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Locale: {}", self.config.app.locale)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Engine: {:?}", self.config.browser.engine)),
                    )
                    .child(div().text_sm().child(format!("Database: {storage}")))
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Config: {}", self.paths.config_file.display())),
                    ),
            )
            .into_any_element()
    }
}
