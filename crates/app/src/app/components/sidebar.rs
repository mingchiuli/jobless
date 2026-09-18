use gpui_kit::base::{StyledExt, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu,
    SidebarMenuItem,
};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div, px};
use rust_i18n::t;

use super::super::{AppView, Page};

impl AppView {
    pub(in crate::app) fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.weak_entity();
        let item = |label: String, page: Page| {
            let weak = weak.clone();
            SidebarMenuItem::new(label)
                .active(self.page == page)
                .on_click(move |_, _, cx| {
                    let _ = weak.update(cx, |this, cx| {
                        this.page = page;
                        cx.notify();
                    });
                })
        };

        Sidebar::new("jobless-sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .w(px(230.))
            .header(
                SidebarHeader::new().child(
                    v_flex()
                        .child(div().font_bold().child("Jobless"))
                        .child(div().text_xs().child(t!("app.subtitle").to_string())),
                ),
            )
            .child(SidebarGroup::new(t!("app.navigation").to_string()).child(
                SidebarMenu::new().children([
                    item(t!("page.accounts").to_string(), Page::Accounts),
                    item(t!("page.tasks").to_string(), Page::Tasks),
                    item(t!("page.browser").to_string(), Page::Browser),
                    item(t!("page.history").to_string(), Page::History),
                    item(t!("page.settings").to_string(), Page::Settings),
                ]),
            ))
            .footer(
                SidebarFooter::new().child(
                    v_flex().gap_2().child(self.theme_switch(cx)).child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                t!("app.version", version = env!("CARGO_PKG_VERSION")).to_string(),
                            ),
                    ),
                ),
            )
    }
}
