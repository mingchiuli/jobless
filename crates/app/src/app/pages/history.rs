use gpui_kit::{AnyElement, Context};

use super::super::components::empty_state::empty_state;
use super::super::{AppView, Page};
use super::page_title;

impl AppView {
    pub(in crate::app) fn history_page(&self, cx: &mut Context<Self>) -> AnyElement {
        empty_state(page_title(Page::History), cx)
    }
}
