//! App-wide preferences, loaded from and saved to the storage backend.

use crate::{
    save_data::{AppSettingsSaveData, SETTINGS_FORMAT_VERSION, Theme},
    storage::Storage,
};

/// App-wide preferences managed as a runtime type.
///
/// Create via [`AppSettings::load`] on startup; use the accessor and mutator methods to read and
/// write settings. Call [`save`] to persist changes. Both load and save are async to support the
/// web IndexedDB backend.
///
/// This type is provided as a Dioxus context at the app root (T09). Because Dioxus context
/// values are typically wrapped in a `Signal`, this type does not need interior mutability.
///
/// [`save`]: AppSettings::save
#[derive(Clone, Debug, PartialEq)]
pub struct AppSettings {
    data: AppSettingsSaveData,
}

impl AppSettings {
    /// Constructs an `AppSettings` from deserialized save data.
    pub fn from_save_data(data: AppSettingsSaveData) -> Self {
        Self { data }
    }

    /// Returns a reference to the underlying save data for persistence.
    pub fn as_save_data(&self) -> &AppSettingsSaveData {
        &self.data
    }

    /// Loads settings from the storage backend, falling back to defaults when no saved data
    /// exists.
    ///
    /// The caller is responsible for applying migration (T05) to the loaded data before passing
    /// it here, if a migration layer is in use.
    pub async fn load<S: Storage>(storage: &S) -> Result<Self, S::Error> {
        match storage.load_settings().await? {
            Some(data) => Ok(Self::from_save_data(data)),
            None => Ok(Self::default()),
        }
    }

    /// Persists the current settings to the storage backend.
    pub async fn save<S: Storage>(&self, storage: &S) -> Result<(), S::Error> {
        storage.save_settings(&self.data).await
    }

    /// Color scheme preference (Dark, Light, or System).
    pub fn theme(&self) -> Theme {
        self.data.theme
    }

    /// Sets the color scheme preference.
    pub fn set_theme(&mut self, theme: Theme) {
        self.data.theme = theme;
    }

    /// When `true`, sets with a past retirement date are excluded from the entire app.
    pub fn ignore_unobtainable_sets(&self) -> bool {
        self.data.ignore_unobtainable_sets
    }

    /// Sets the `ignore_unobtainable_sets` preference.
    pub fn set_ignore_unobtainable_sets(&mut self, value: bool) {
        self.data.ignore_unobtainable_sets = value;
    }

    /// When `true`, cards with source `"Premium Mission"` are excluded from the entire app.
    pub fn ignore_premium_mission(&self) -> bool {
        self.data.ignore_premium_mission
    }

    /// Sets the `ignore_premium_mission` preference.
    pub fn set_ignore_premium_mission(&mut self, value: bool) {
        self.data.ignore_premium_mission = value;
    }

    /// When `true`, cards with source `"Gold Shop"` are excluded from the entire app.
    pub fn ignore_gold_shop(&self) -> bool {
        self.data.ignore_gold_shop
    }

    /// Sets the `ignore_gold_shop` preference.
    pub fn set_ignore_gold_shop(&mut self, value: bool) {
        self.data.ignore_gold_shop = value;
    }

    /// When `true`, duplicate card version groups are treated as a single logical card
    /// throughout the entire app. See DESIGN.md §Merge duplicate printings.
    pub fn merge_duplicate_printings(&self) -> bool {
        self.data.merge_duplicate_printings
    }

    /// Sets the `merge_duplicate_printings` preference.
    pub fn set_merge_duplicate_printings(&mut self, value: bool) {
        self.data.merge_duplicate_printings = value;
    }

    /// Names of the currently active profiles.
    pub fn active_profile_names(&self) -> &[String] {
        &self.data.active_profile_names
    }

    /// Sets the active profile names. Should always be a non-empty subset of the known profile
    /// names stored in `ProfilesSaveData`.
    pub fn set_active_profile_names(&mut self, names: Vec<String>) {
        self.data.active_profile_names = names;
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            data: AppSettingsSaveData {
                format_version: SETTINGS_FORMAT_VERSION,
                theme: Theme::System,
                ignore_unobtainable_sets: false,
                ignore_premium_mission: false,
                ignore_gold_shop: false,
                merge_duplicate_printings: false,
                active_profile_names: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_match_spec() {
        let s = AppSettings::default();
        assert_eq!(s.theme(), Theme::System);
        assert!(!s.ignore_unobtainable_sets());
        assert!(!s.ignore_premium_mission());
        assert!(!s.ignore_gold_shop());
        assert!(!s.merge_duplicate_printings());
        assert!(s.active_profile_names().is_empty());
    }

    #[test]
    fn setters_update_state() {
        let mut s = AppSettings::default();
        s.set_theme(Theme::Dark);
        s.set_ignore_unobtainable_sets(true);
        s.set_ignore_premium_mission(true);
        s.set_ignore_gold_shop(true);
        s.set_merge_duplicate_printings(true);
        s.set_active_profile_names(vec!["Main".to_string(), "Alt".to_string()]);

        assert_eq!(s.theme(), Theme::Dark);
        assert!(s.ignore_unobtainable_sets());
        assert!(s.ignore_premium_mission());
        assert!(s.ignore_gold_shop());
        assert!(s.merge_duplicate_printings());
        assert_eq!(s.active_profile_names(), &["Main", "Alt"]);
    }

    #[test]
    fn round_trip_save_data() {
        let mut s = AppSettings::default();
        s.set_theme(Theme::Light);
        s.set_ignore_unobtainable_sets(true);

        let data = s.as_save_data().clone();
        let s2 = AppSettings::from_save_data(data);
        assert_eq!(s, s2);
    }
}
