//! Central collection state: all profiles, owned counts, and active-profile tracking.
//!
//! [`ProfileStore`] is the primary runtime type for collection data. It owns the storage backend
//! and is provided as a Dioxus context at the app root (T09), wrapped in a `Signal`.
//!
//! ## Auto-save
//!
//! `ProfileStore` tracks a dirty flag that is set whenever collection data changes. It does not
//! spawn its own background timer — the Dioxus integration layer (T09) is responsible for
//! debouncing saves: after any write, T09 should schedule a [`ProfileStore::save`] call
//! 2 seconds after the last mutation. Use [`ProfileStore::needs_save`] to check whether a
//! save is pending.

use std::collections::HashMap;

use crate::{
    MigrationError, migrate_profiles,
    save_data::{CardVersionId, PROFILES_FORMAT_VERSION, ProfileData, ProfilesSaveData},
    storage::Storage,
};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`ProfileStore`] operations.
#[derive(Debug, thiserror::Error)]
pub enum ProfileStoreError<E> {
    /// A storage backend operation failed.
    #[error("storage error: {0}")]
    Storage(E),

    /// The requested profile name is already in use.
    #[error("profile name already taken: {0}")]
    NameTaken(String),

    /// No profile with the given name exists.
    #[error("profile not found: {0}")]
    NotFound(String),

    /// The operation would delete or deactivate the only remaining profile or active profile
    /// when the constraint requires at least one to remain.
    #[error("cannot remove the last profile")]
    OnlyProfile,

    /// The operation would deactivate the last active profile.
    #[error("cannot deactivate the only active profile")]
    OnlyActive,

    /// Save data was written by a newer version of the app.
    #[error("migration error: {0}")]
    Migration(MigrationError),
}

// ---------------------------------------------------------------------------
// ProfileStore
// ---------------------------------------------------------------------------

/// Central runtime type for all collection data.
///
/// Owns all profiles, their owned-count maps, the active-profile set, and the storage backend.
/// Mutation methods mark the store dirty; callers are responsible for calling [`save`] (T09
/// handles the 2-second debounce). See module-level docs for the auto-save contract.
///
/// [`save`]: ProfileStore::save
#[derive(Clone, Debug)]
pub struct ProfileStore<S: Storage> {
    data: ProfilesSaveData,
    storage: S,
    dirty: bool,
}

impl<S: Storage + Clone> ProfileStore<S> {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// Creates a new, empty store (first-run state). No profiles exist; no save has been written.
    pub fn new(storage: S) -> Self {
        Self {
            data: ProfilesSaveData {
                format_version: PROFILES_FORMAT_VERSION,
                profiles: Vec::new(),
                primary_profile_name: String::new(),
                active_profile_names: Vec::new(),
            },
            storage,
            dirty: false,
        }
    }

    /// Loads data from the storage backend, returning an empty store when no data has been saved.
    ///
    /// Applies the migration layer before returning. After loading, the active-profile list is
    /// validated: stale names are removed, and if the result is empty (e.g. data written before
    /// `active_profile_names` was introduced), the primary profile is activated by default.
    pub async fn load(storage: S) -> Result<Self, ProfileStoreError<S::Error>> {
        let raw = storage
            .load_profiles()
            .await
            .map_err(ProfileStoreError::Storage)?;

        let Some(raw) = raw else {
            return Ok(Self::new(storage));
        };

        let mut data = migrate_profiles(raw).map_err(ProfileStoreError::Migration)?;

        // Validate active-profile list: remove any names that no longer exist in profiles.
        data.active_profile_names
            .retain(|n| data.profiles.iter().any(|p| p.name == *n));

        // Default to primary profile when the list is empty (first boot, migration, etc.).
        if data.active_profile_names.is_empty() && !data.profiles.is_empty() {
            data.active_profile_names
                .push(data.primary_profile_name.clone());
        }

        Ok(Self {
            data,
            storage,
            dirty: false,
        })
    }

    /// Persists the current state to the storage backend and clears the dirty flag.
    pub async fn save(&mut self) -> Result<(), ProfileStoreError<S::Error>> {
        self.storage
            .save_profiles(&self.data)
            .await
            .map_err(ProfileStoreError::Storage)?;
        self.dirty = false;
        Ok(())
    }

    /// Returns `true` when unsaved changes exist. The Dioxus integration layer uses this to
    /// schedule the 2-second debounced auto-save.
    pub fn needs_save(&self) -> bool {
        self.dirty
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    /// Returns `true` when no profiles have been created yet (first-run state).
    pub fn is_first_run(&self) -> bool {
        self.data.profiles.is_empty()
    }

    /// All profiles in creation order.
    pub fn profiles(&self) -> &[ProfileData] {
        &self.data.profiles
    }

    /// Name of the designated primary profile, or an empty string on first run.
    pub fn primary_profile_name(&self) -> &str {
        &self.data.primary_profile_name
    }

    /// Names of the currently active profiles.
    pub fn active_profile_names(&self) -> &[String] {
        &self.data.active_profile_names
    }

    /// Owned count for a specific card version in a specific profile. Returns `0` when the
    /// profile has no entry for that card.
    pub fn owned_count(&self, profile_name: &str, card_id: CardVersionId) -> u32 {
        self.data
            .profiles
            .iter()
            .find(|p| p.name == profile_name)
            .and_then(|p| p.owned_counts.get(&card_id).copied())
            .unwrap_or(0)
    }

    /// Sum of owned counts for a card version across all currently active profiles.
    pub fn aggregate_count(&self, card_id: CardVersionId) -> u32 {
        self.data
            .active_profile_names
            .iter()
            .map(|name| self.owned_count(name, card_id))
            .sum()
    }

    // ------------------------------------------------------------------
    // Mutations — owned counts
    // ------------------------------------------------------------------

    /// Sets the owned count for a card version in a profile.
    ///
    /// When `count` is zero the entry is removed from the map (absent entries implicitly have a
    /// count of zero). No-ops silently when `count` equals the current stored value.
    pub fn set_owned_count(
        &mut self,
        profile_name: &str,
        card_id: CardVersionId,
        count: u32,
    ) -> Result<(), ProfileStoreError<S::Error>> {
        let profile = self
            .data
            .profiles
            .iter_mut()
            .find(|p| p.name == profile_name)
            .ok_or_else(|| ProfileStoreError::NotFound(profile_name.to_string()))?;

        let current = profile.owned_counts.get(&card_id).copied().unwrap_or(0);
        if current == count {
            return Ok(());
        }

        if count == 0 {
            profile.owned_counts.remove(&card_id);
        } else {
            profile.owned_counts.insert(card_id, count);
        }
        self.dirty = true;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Mutations — profile management
    // ------------------------------------------------------------------

    /// Creates a new profile with the given name.
    ///
    /// The first profile created becomes both the primary profile and the sole active profile.
    /// Returns [`ProfileStoreError::NameTaken`] if a profile with that name already exists.
    pub fn create_profile(&mut self, name: String) -> Result<(), ProfileStoreError<S::Error>> {
        if self.data.profiles.iter().any(|p| p.name == name) {
            return Err(ProfileStoreError::NameTaken(name));
        }

        let is_first = self.data.profiles.is_empty();
        self.data.profiles.push(ProfileData {
            name: name.clone(),
            owned_counts: HashMap::new(),
        });

        if is_first {
            self.data.primary_profile_name = name.clone();
            self.data.active_profile_names = vec![name];
        }

        self.dirty = true;
        Ok(())
    }

    /// Renames an existing profile.
    ///
    /// Updates the primary-profile name and active-profile list when the renamed profile appears
    /// in either. Returns [`ProfileStoreError::NameTaken`] if `new_name` is already in use, or
    /// [`ProfileStoreError::NotFound`] if no profile named `old_name` exists.
    pub fn rename_profile(
        &mut self,
        old_name: &str,
        new_name: String,
    ) -> Result<(), ProfileStoreError<S::Error>> {
        if self.data.profiles.iter().any(|p| p.name == new_name) {
            return Err(ProfileStoreError::NameTaken(new_name));
        }

        let profile = self
            .data
            .profiles
            .iter_mut()
            .find(|p| p.name == old_name)
            .ok_or_else(|| ProfileStoreError::NotFound(old_name.to_string()))?;

        profile.name = new_name.clone();

        if self.data.primary_profile_name == old_name {
            self.data.primary_profile_name = new_name.clone();
        }

        for n in &mut self.data.active_profile_names {
            if n == old_name {
                *n = new_name.clone();
            }
        }

        self.dirty = true;
        Ok(())
    }

    /// Deletes a profile.
    ///
    /// Returns [`ProfileStoreError::OnlyProfile`] if this is the last remaining profile.
    ///
    /// **Primary promotion**: when the deleted profile was the primary, the remaining profile
    /// with the largest total owned count is promoted. Ties are broken by position (last wins,
    /// matching Rust's [`Iterator::max_by_key`] tie-breaking behaviour, which is arbitrary per
    /// spec).
    ///
    /// **Active-set update**: when the deleted profile was active, it is removed from the active
    /// set. If that leaves the active set empty, the (possibly newly promoted) primary profile
    /// is activated.
    pub fn delete_profile(&mut self, name: &str) -> Result<(), ProfileStoreError<S::Error>> {
        if self.data.profiles.len() == 1 {
            return Err(ProfileStoreError::OnlyProfile);
        }

        let pos = self
            .data
            .profiles
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| ProfileStoreError::NotFound(name.to_string()))?;

        self.data.profiles.remove(pos);

        if self.data.primary_profile_name == name {
            let new_primary = self
                .data
                .profiles
                .iter()
                .max_by_key(|p| p.owned_counts.values().sum::<u32>())
                .map(|p| p.name.clone())
                .unwrap_or_default();
            self.data.primary_profile_name = new_primary;
        }

        self.data.active_profile_names.retain(|n| n != name);
        if self.data.active_profile_names.is_empty() {
            self.data
                .active_profile_names
                .push(self.data.primary_profile_name.clone());
        }

        self.dirty = true;
        Ok(())
    }

    /// Designates the named profile as the primary profile.
    ///
    /// Returns [`ProfileStoreError::NotFound`] if no profile with that name exists.
    pub fn set_primary(&mut self, name: &str) -> Result<(), ProfileStoreError<S::Error>> {
        if !self.data.profiles.iter().any(|p| p.name == name) {
            return Err(ProfileStoreError::NotFound(name.to_string()));
        }
        self.data.primary_profile_name = name.to_string();
        self.dirty = true;
        Ok(())
    }

    /// Adds a profile to the active set. No-ops if it is already active.
    ///
    /// Returns [`ProfileStoreError::NotFound`] if no profile with that name exists.
    pub fn activate_profile(&mut self, name: &str) -> Result<(), ProfileStoreError<S::Error>> {
        if !self.data.profiles.iter().any(|p| p.name == name) {
            return Err(ProfileStoreError::NotFound(name.to_string()));
        }
        if !self.data.active_profile_names.iter().any(|n| n == name) {
            self.data.active_profile_names.push(name.to_string());
            self.dirty = true;
        }
        Ok(())
    }

    /// Removes a profile from the active set.
    ///
    /// Returns [`ProfileStoreError::OnlyActive`] if this is the only active profile. No-ops if
    /// the profile is already inactive.
    ///
    /// Returns [`ProfileStoreError::NotFound`] if no profile with that name exists.
    pub fn deactivate_profile(&mut self, name: &str) -> Result<(), ProfileStoreError<S::Error>> {
        if !self.data.profiles.iter().any(|p| p.name == name) {
            return Err(ProfileStoreError::NotFound(name.to_string()));
        }
        if !self.data.active_profile_names.iter().any(|n| n == name) {
            return Ok(());
        }
        if self.data.active_profile_names.len() == 1 {
            return Err(ProfileStoreError::OnlyActive);
        }
        self.data.active_profile_names.retain(|n| n != name);
        self.dirty = true;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::PROFILES_FORMAT_VERSION;

    // A minimal in-memory Storage stub for unit tests.
    #[derive(Clone, Debug, Default)]
    struct MemStorage {
        profiles: std::rc::Rc<std::cell::RefCell<Option<ProfilesSaveData>>>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("mem storage error")]
    struct MemError;

    impl Storage for MemStorage {
        type Error = MemError;

        async fn load_profiles(&self) -> Result<Option<ProfilesSaveData>, Self::Error> {
            Ok(self.profiles.borrow().clone())
        }

        async fn save_profiles(&self, data: &ProfilesSaveData) -> Result<(), Self::Error> {
            *self.profiles.borrow_mut() = Some(data.clone());
            Ok(())
        }

        async fn load_settings(
            &self,
        ) -> Result<Option<crate::save_data::AppSettingsSaveData>, Self::Error> {
            Ok(None)
        }

        async fn save_settings(
            &self,
            _data: &crate::save_data::AppSettingsSaveData,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn load_saved_queries(
            &self,
        ) -> Result<Option<crate::save_data::SavedQueriesSaveData>, Self::Error> {
            Ok(None)
        }

        async fn save_saved_queries(
            &self,
            _data: &crate::save_data::SavedQueriesSaveData,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn empty_store() -> ProfileStore<MemStorage> {
        ProfileStore::new(MemStorage::default())
    }

    fn store_with_profiles(names: &[&str]) -> ProfileStore<MemStorage> {
        let mut store = empty_store();
        for name in names {
            store.create_profile(name.to_string()).unwrap();
        }
        store.dirty = false; // reset to test dirty-setting in isolation
        store
    }

    // --- first-run state ---

    #[test]
    fn new_store_is_first_run() {
        let store = empty_store();
        assert!(store.is_first_run());
        assert!(store.profiles().is_empty());
        assert!(store.primary_profile_name().is_empty());
        assert!(store.active_profile_names().is_empty());
        assert!(!store.needs_save());
    }

    // --- create_profile ---

    #[test]
    fn create_first_profile_sets_primary_and_active() {
        let mut store = empty_store();
        store.create_profile("Main".to_string()).unwrap();

        assert_eq!(store.profiles().len(), 1);
        assert_eq!(store.primary_profile_name(), "Main");
        assert_eq!(store.active_profile_names(), &["Main"]);
        assert!(!store.is_first_run());
        assert!(store.needs_save());
    }

    #[test]
    fn create_second_profile_does_not_change_primary_or_active() {
        let mut store = store_with_profiles(&["Main"]);
        store.create_profile("Alt".to_string()).unwrap();

        assert_eq!(store.primary_profile_name(), "Main");
        assert_eq!(store.active_profile_names(), &["Main"]);
        assert_eq!(store.profiles().len(), 2);
    }

    #[test]
    fn create_duplicate_name_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store.create_profile("Main".to_string()).unwrap_err();
        assert!(matches!(err, ProfileStoreError::NameTaken(n) if n == "Main"));
    }

    // --- rename_profile ---

    #[test]
    fn rename_updates_profile_primary_and_active() {
        let mut store = store_with_profiles(&["Main"]);
        store.rename_profile("Main", "Renamed".to_string()).unwrap();

        assert_eq!(store.profiles()[0].name, "Renamed");
        assert_eq!(store.primary_profile_name(), "Renamed");
        assert_eq!(store.active_profile_names(), &["Renamed"]);
        assert!(store.needs_save());
    }

    #[test]
    fn rename_non_primary_does_not_change_primary() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.rename_profile("Alt", "Other".to_string()).unwrap();

        assert_eq!(store.primary_profile_name(), "Main");
        assert_eq!(store.profiles()[1].name, "Other");
    }

    #[test]
    fn rename_to_existing_name_fails() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        let err = store.rename_profile("Main", "Alt".to_string()).unwrap_err();
        assert!(matches!(err, ProfileStoreError::NameTaken(_)));
    }

    #[test]
    fn rename_nonexistent_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store
            .rename_profile("Ghost", "New".to_string())
            .unwrap_err();
        assert!(matches!(err, ProfileStoreError::NotFound(_)));
    }

    // --- delete_profile ---

    #[test]
    fn delete_only_profile_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store.delete_profile("Main").unwrap_err();
        assert!(matches!(err, ProfileStoreError::OnlyProfile));
    }

    #[test]
    fn delete_nonprimary_inactive_profile() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.delete_profile("Alt").unwrap();

        assert_eq!(store.profiles().len(), 1);
        assert_eq!(store.primary_profile_name(), "Main");
        assert_eq!(store.active_profile_names(), &["Main"]);
        assert!(store.needs_save());
    }

    #[test]
    fn delete_primary_promotes_profile_with_most_cards() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        // Give Alt one card so it has the highest count.
        store.set_owned_count("Alt", 0, 5).unwrap();
        store.delete_profile("Main").unwrap();

        assert_eq!(store.primary_profile_name(), "Alt");
    }

    #[test]
    fn delete_primary_with_no_cards_promotes_arbitrarily() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.delete_profile("Main").unwrap();
        // With no owned cards, any non-empty primary is acceptable.
        assert!(!store.primary_profile_name().is_empty());
        assert_eq!(store.primary_profile_name(), "Alt");
    }

    #[test]
    fn delete_active_profile_activates_primary_when_set_empty() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        // Main is active; activate Main and mark Main as active only.
        // (store_with_profiles leaves Main as the only active profile)
        store.delete_profile("Main").unwrap();

        // After deletion, Alt becomes primary and active set should not be empty.
        assert_eq!(store.active_profile_names(), &["Alt"]);
    }

    #[test]
    fn delete_nonexistent_fails() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        let err = store.delete_profile("Ghost").unwrap_err();
        assert!(matches!(err, ProfileStoreError::NotFound(_)));
    }

    // --- set_primary ---

    #[test]
    fn set_primary_updates_designation() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.set_primary("Alt").unwrap();
        assert_eq!(store.primary_profile_name(), "Alt");
        assert!(store.needs_save());
    }

    #[test]
    fn set_primary_nonexistent_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store.set_primary("Ghost").unwrap_err();
        assert!(matches!(err, ProfileStoreError::NotFound(_)));
    }

    // --- activate / deactivate ---

    #[test]
    fn activate_inactive_profile() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.activate_profile("Alt").unwrap();
        assert!(store.active_profile_names().contains(&"Alt".to_string()));
        assert!(store.needs_save());
    }

    #[test]
    fn activate_already_active_is_noop() {
        let mut store = store_with_profiles(&["Main"]);
        store.activate_profile("Main").unwrap();
        // dirty should not be set for a no-op
        assert!(!store.needs_save());
        assert_eq!(store.active_profile_names().len(), 1);
    }

    #[test]
    fn activate_nonexistent_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store.activate_profile("Ghost").unwrap_err();
        assert!(matches!(err, ProfileStoreError::NotFound(_)));
    }

    #[test]
    fn deactivate_one_of_two_active() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.activate_profile("Alt").unwrap();
        store.dirty = false;

        store.deactivate_profile("Alt").unwrap();
        assert!(!store.active_profile_names().contains(&"Alt".to_string()));
        assert!(store.needs_save());
    }

    #[test]
    fn deactivate_only_active_fails() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        let err = store.deactivate_profile("Main").unwrap_err();
        assert!(matches!(err, ProfileStoreError::OnlyActive));
    }

    #[test]
    fn deactivate_already_inactive_is_noop() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.deactivate_profile("Alt").unwrap(); // Alt is already inactive
        assert!(!store.needs_save());
    }

    #[test]
    fn deactivate_nonexistent_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store.deactivate_profile("Ghost").unwrap_err();
        assert!(matches!(err, ProfileStoreError::NotFound(_)));
    }

    // --- set_owned_count / aggregate_count ---

    #[test]
    fn set_and_read_owned_count() {
        let mut store = store_with_profiles(&["Main"]);
        store.set_owned_count("Main", 42, 3).unwrap();
        assert_eq!(store.owned_count("Main", 42), 3);
        assert!(store.needs_save());
    }

    #[test]
    fn set_zero_removes_entry() {
        let mut store = store_with_profiles(&["Main"]);
        store.set_owned_count("Main", 42, 3).unwrap();
        store.set_owned_count("Main", 42, 0).unwrap();
        assert_eq!(store.owned_count("Main", 42), 0);
        assert!(
            store
                .data
                .profiles
                .iter()
                .find(|p| p.name == "Main")
                .unwrap()
                .owned_counts
                .is_empty()
        );
    }

    #[test]
    fn set_same_value_is_noop() {
        let mut store = store_with_profiles(&["Main"]);
        store.set_owned_count("Main", 42, 3).unwrap();
        store.dirty = false;
        store.set_owned_count("Main", 42, 3).unwrap();
        assert!(!store.needs_save());
    }

    #[test]
    fn set_owned_count_unknown_profile_fails() {
        let mut store = store_with_profiles(&["Main"]);
        let err = store.set_owned_count("Ghost", 0, 1).unwrap_err();
        assert!(matches!(err, ProfileStoreError::NotFound(_)));
    }

    #[test]
    fn absent_entry_reads_as_zero() {
        let store = store_with_profiles(&["Main"]);
        assert_eq!(store.owned_count("Main", 999), 0);
    }

    #[test]
    fn aggregate_count_sums_active_profiles() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        store.activate_profile("Alt").unwrap();
        store.set_owned_count("Main", 0, 2).unwrap();
        store.set_owned_count("Alt", 0, 3).unwrap();
        assert_eq!(store.aggregate_count(0), 5);
    }

    #[test]
    fn aggregate_count_excludes_inactive_profiles() {
        let mut store = store_with_profiles(&["Main", "Alt"]);
        // Alt is not active; only Main is.
        store.set_owned_count("Main", 0, 2).unwrap();
        store.set_owned_count("Alt", 0, 10).unwrap();
        assert_eq!(store.aggregate_count(0), 2);
    }

    // --- load / save round-trip ---

    #[tokio::test]
    async fn load_empty_storage_returns_new_store() {
        let store = ProfileStore::load(MemStorage::default()).await.unwrap();
        assert!(store.is_first_run());
    }

    #[tokio::test]
    async fn save_then_load_round_trip() {
        let storage = MemStorage::default();
        let mut store = ProfileStore::new(storage.clone());
        store.create_profile("Main".to_string()).unwrap();
        store.set_owned_count("Main", 7, 4).unwrap();
        store.save().await.unwrap();
        assert!(!store.needs_save());

        let loaded = ProfileStore::load(storage).await.unwrap();
        assert_eq!(loaded.owned_count("Main", 7), 4);
        assert_eq!(loaded.primary_profile_name(), "Main");
        assert_eq!(loaded.active_profile_names(), &["Main"]);
    }

    #[tokio::test]
    async fn load_heals_empty_active_profile_names() {
        // Simulate a save file that pre-dates the active_profile_names field
        // by loading data with an empty active list.
        let storage = MemStorage::default();
        let data = ProfilesSaveData {
            format_version: PROFILES_FORMAT_VERSION,
            profiles: vec![ProfileData {
                name: "Main".to_string(),
                owned_counts: HashMap::new(),
            }],
            primary_profile_name: "Main".to_string(),
            active_profile_names: Vec::new(),
        };
        *storage.profiles.borrow_mut() = Some(data);

        let loaded = ProfileStore::load(storage).await.unwrap();
        assert_eq!(loaded.active_profile_names(), &["Main"]);
    }
}
