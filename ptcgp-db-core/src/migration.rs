//! Data format migration for save data loaded from storage.
//!
//! Each save data type carries a `format_version` field. When the app loads data, it passes it
//! through the corresponding `migrate_*` function, which upgrades the data in-place through each
//! version boundary to the current version.
//!
//! ## Adding a new version
//!
//! 1. Bump the relevant `*_FORMAT_VERSION` constant in [`crate::save_data`].
//! 2. Write a private `migrate_<type>_N_to_M` function that takes the V(N) struct and returns the
//!    V(M) struct. If the shape changed, add a versioned snapshot type (e.g. `ProfilesSaveDataV1`).
//! 3. Chain it into the public `migrate_*` function below with
//!    `if data.format_version < M { data = migrate_<type>_N_to_M(data); }`.

use crate::save_data::{
    AppSettingsSaveData, PROFILES_FORMAT_VERSION, ProfilesSaveData, QUERIES_FORMAT_VERSION,
    SETTINGS_FORMAT_VERSION, SavedQueriesSaveData,
};

/// Error returned by the migration functions.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    /// The data's `format_version` is newer than this build of the app understands.
    ///
    /// This can occur when loading save data written by a newer version of the app on another
    /// device, or after a downgrade.
    #[error("data format version {found} is newer than the current version {current}")]
    FutureVersion { found: u32, current: u32 },
}

/// Migrate [`ProfilesSaveData`] to the current format version.
///
/// Returns `Ok(data)` unchanged when `data.format_version == `[`PROFILES_FORMAT_VERSION`].
/// Returns [`MigrationError::FutureVersion`] when the data was written by a newer app build.
pub fn migrate_profiles(data: ProfilesSaveData) -> Result<ProfilesSaveData, MigrationError> {
    if data.format_version > PROFILES_FORMAT_VERSION {
        return Err(MigrationError::FutureVersion {
            found: data.format_version,
            current: PROFILES_FORMAT_VERSION,
        });
    }
    // V1 is the initial version; no migrations to apply.
    // Future: if data.format_version < 2 { data = migrate_profiles_1_to_2(data); }
    Ok(ProfilesSaveData {
        format_version: PROFILES_FORMAT_VERSION,
        ..data
    })
}

/// Migrate [`AppSettingsSaveData`] to the current format version.
///
/// Returns `Ok(data)` unchanged when `data.format_version == `[`SETTINGS_FORMAT_VERSION`].
/// Returns [`MigrationError::FutureVersion`] when the data was written by a newer app build.
pub fn migrate_settings(data: AppSettingsSaveData) -> Result<AppSettingsSaveData, MigrationError> {
    if data.format_version > SETTINGS_FORMAT_VERSION {
        return Err(MigrationError::FutureVersion {
            found: data.format_version,
            current: SETTINGS_FORMAT_VERSION,
        });
    }
    // V1 is the initial version; no migrations to apply.
    // Future: if data.format_version < 2 { data = migrate_settings_1_to_2(data); }
    Ok(AppSettingsSaveData {
        format_version: SETTINGS_FORMAT_VERSION,
        ..data
    })
}

/// Migrate [`SavedQueriesSaveData`] to the current format version.
///
/// Returns `Ok(data)` unchanged when `data.format_version == `[`QUERIES_FORMAT_VERSION`].
/// Returns [`MigrationError::FutureVersion`] when the data was written by a newer app build.
pub fn migrate_saved_queries(
    data: SavedQueriesSaveData,
) -> Result<SavedQueriesSaveData, MigrationError> {
    if data.format_version > QUERIES_FORMAT_VERSION {
        return Err(MigrationError::FutureVersion {
            found: data.format_version,
            current: QUERIES_FORMAT_VERSION,
        });
    }
    // V1 is the initial version; no migrations to apply.
    // Future: if data.format_version < 2 { data = migrate_saved_queries_1_to_2(data); }
    Ok(SavedQueriesSaveData {
        format_version: QUERIES_FORMAT_VERSION,
        ..data
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::*;

    // --- ProfilesSaveData ---

    #[test]
    fn migrate_profiles_v1_passthrough() {
        let data = ProfilesSaveData {
            format_version: 1,
            profiles: vec![ProfileData {
                name: "Main".to_string(),
                owned_counts: Default::default(),
            }],
            primary_profile_name: "Main".to_string(),
            active_profile_names: vec!["Main".to_string()],
        };
        let migrated = migrate_profiles(data.clone()).unwrap();
        assert_eq!(migrated, data);
    }

    #[test]
    fn migrate_profiles_future_version_errors() {
        let data = ProfilesSaveData {
            format_version: 9999,
            profiles: vec![],
            primary_profile_name: String::new(),
            active_profile_names: vec![],
        };
        assert!(matches!(
            migrate_profiles(data),
            Err(MigrationError::FutureVersion {
                found: 9999,
                current: PROFILES_FORMAT_VERSION
            })
        ));
    }

    // --- AppSettingsSaveData ---

    #[test]
    fn migrate_settings_v1_passthrough() {
        let data = AppSettingsSaveData {
            format_version: 1,
            theme: Theme::System,
            ignore_unobtainable_sets: false,
            ignore_premium_mission: false,
            ignore_gold_shop: false,
            merge_duplicate_printings: false,
        };
        let migrated = migrate_settings(data.clone()).unwrap();
        assert_eq!(migrated, data);
    }

    #[test]
    fn migrate_settings_future_version_errors() {
        let data = AppSettingsSaveData {
            format_version: 9999,
            theme: Theme::Dark,
            ignore_unobtainable_sets: false,
            ignore_premium_mission: false,
            ignore_gold_shop: false,
            merge_duplicate_printings: false,
        };
        assert!(matches!(
            migrate_settings(data),
            Err(MigrationError::FutureVersion {
                found: 9999,
                current: SETTINGS_FORMAT_VERSION
            })
        ));
    }

    // --- SavedQueriesSaveData ---

    #[test]
    fn migrate_saved_queries_v1_passthrough() {
        let data = SavedQueriesSaveData {
            format_version: 1,
            queries: vec![SavedQuery {
                name: "All ex".to_string(),
                config: FilterConfig {
                    ex: Some(true),
                    ..Default::default()
                },
            }],
        };
        let migrated = migrate_saved_queries(data.clone()).unwrap();
        assert_eq!(migrated, data);
    }

    #[test]
    fn migrate_saved_queries_future_version_errors() {
        let data = SavedQueriesSaveData {
            format_version: 9999,
            queries: vec![],
        };
        assert!(matches!(
            migrate_saved_queries(data),
            Err(MigrationError::FutureVersion {
                found: 9999,
                current: QUERIES_FORMAT_VERSION
            })
        ));
    }
}
