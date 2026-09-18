mod accounts;
mod browser;
mod history;
mod settings;
mod tasks;

use rust_i18n::t;

use super::Page;

pub(super) fn page_title(page: Page) -> String {
    match page {
        Page::Accounts => t!("page.accounts").to_string(),
        Page::Tasks => t!("page.tasks").to_string(),
        Page::Browser => t!("page.browser").to_string(),
        Page::History => t!("page.history").to_string(),
        Page::Settings => t!("page.settings").to_string(),
    }
}
