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

    /// Returns a reference to the storage backend. Used by the Dioxus integration layer to
    /// perform an async save without holding a write lock on the `Signal<ProfileStore>`.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Returns a snapshot of the current save data. Pair with [`mark_clean`] after a successful
    /// external save to avoid holding a write lock across an await point.
    ///
    /// [`mark_clean`]: ProfileStore::mark_clean
    pub fn save_data_snapshot(&self) -> &ProfilesSaveData {
        &self.data
    }

    /// Clears the dirty flag without persisting. Called by the Dioxus integration layer after a
    /// successful async save performed outside the write lock.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
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

    /// Replaces all owned counts for an existing profile with `counts`.
    ///
    /// Zero-valued entries are stripped so the map stays compact (absent entries are
    /// implicitly zero). The entire map is replaced atomically and the dirty flag is set.
    /// Returns [`ProfileStoreError::NotFound`] if no profile with that name exists.
    pub fn replace_profile_counts(
        &mut self,
        profile_name: &str,
        counts: HashMap<CardVersionId, u32>,
    ) -> Result<(), ProfileStoreError<S::Error>> {
        let profile = self
            .data
            .profiles
            .iter_mut()
            .find(|p| p.name == profile_name)
            .ok_or_else(|| ProfileStoreError::NotFound(profile_name.to_string()))?;
        profile.owned_counts = counts.into_iter().filter(|(_, v)| *v > 0).collect();
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

#[cfg(test)]
mod tests;
