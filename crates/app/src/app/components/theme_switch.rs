use gpui_kit::component::Sizable;
use gpui_kit::component::button::*;
use gpui_kit::{Context, IntoElement};

use super::super::AppView;

impl AppView {
    pub(in crate::app) fn theme_switch(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("theme-switch")
            .small()
            .label(self.theme_preference.label())
            .on_click(cx.listener(|this, _, window, cx| this.cycle_theme(window, cx)))
    }
}
