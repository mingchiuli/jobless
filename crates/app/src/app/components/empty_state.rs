use gpui_kit::base::{StyledExt, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use rust_i18n::t;

use super::super::AppView;

pub(in crate::app) fn empty_state(title: String, cx: &mut Context<AppView>) -> AnyElement {
    v_flex()
        .gap_4()
        .child(div().font_bold().text_xl().child(title))
        .child(
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .rounded(cx.theme().radius)
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("common.empty").to_string()),
                ),
        )
        .into_any_element()
}
