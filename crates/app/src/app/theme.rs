use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Window};
use rust_i18n::t;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub(super) fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    pub(super) fn label(self) -> String {
        match self {
            Self::System => t!("theme.system").to_string(),
            Self::Light => t!("theme.light").to_string(),
            Self::Dark => t!("theme.dark").to_string(),
        }
    }

    pub(super) fn apply(self, window: Option<&mut Window>, cx: &mut App) {
        match self {
            Self::System => Theme::sync_system_appearance(window, cx),
            Self::Light => Theme::change(ThemeMode::Light, window, cx),
            Self::Dark => Theme::change(ThemeMode::Dark, window, cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ThemePreference;

    #[test]
    fn preference_cycles_through_system_light_and_dark() {
        assert_eq!(ThemePreference::System.next(), ThemePreference::Light);
        assert_eq!(ThemePreference::Light.next(), ThemePreference::Dark);
        assert_eq!(ThemePreference::Dark.next(), ThemePreference::System);
    }
}
