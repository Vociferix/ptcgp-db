use super::*;
use crate::save_data::CardVersionId;

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
    store.set_owned_count("Alt", CardVersionId(0), 5).unwrap();
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
    store.set_owned_count("Main", CardVersionId(42), 3).unwrap();
    assert_eq!(store.owned_count("Main", CardVersionId(42)), 3);
    assert!(store.needs_save());
}

#[test]
fn set_zero_removes_entry() {
    let mut store = store_with_profiles(&["Main"]);
    store.set_owned_count("Main", CardVersionId(42), 3).unwrap();
    store.set_owned_count("Main", CardVersionId(42), 0).unwrap();
    assert_eq!(store.owned_count("Main", CardVersionId(42)), 0);
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
    store.set_owned_count("Main", CardVersionId(42), 3).unwrap();
    store.dirty = false;
    store.set_owned_count("Main", CardVersionId(42), 3).unwrap();
    assert!(!store.needs_save());
}

#[test]
fn set_owned_count_unknown_profile_fails() {
    let mut store = store_with_profiles(&["Main"]);
    let err = store.set_owned_count("Ghost", CardVersionId(0), 1).unwrap_err();
    assert!(matches!(err, ProfileStoreError::NotFound(_)));
}

#[test]
fn absent_entry_reads_as_zero() {
    let store = store_with_profiles(&["Main"]);
    assert_eq!(store.owned_count("Main", CardVersionId(999)), 0);
}

#[test]
fn aggregate_count_sums_active_profiles() {
    let mut store = store_with_profiles(&["Main", "Alt"]);
    store.activate_profile("Alt").unwrap();
    store.set_owned_count("Main", CardVersionId(0), 2).unwrap();
    store.set_owned_count("Alt", CardVersionId(0), 3).unwrap();
    assert_eq!(store.aggregate_count(CardVersionId(0)), 5);
}

#[test]
fn aggregate_count_excludes_inactive_profiles() {
    let mut store = store_with_profiles(&["Main", "Alt"]);
    // Alt is not active; only Main is.
    store.set_owned_count("Main", CardVersionId(0), 2).unwrap();
    store.set_owned_count("Alt", CardVersionId(0), 10).unwrap();
    assert_eq!(store.aggregate_count(CardVersionId(0)), 2);
}

// --- replace_profile_counts ---

#[test]
fn replace_counts_overwrites_existing_map() {
    let mut store = store_with_profiles(&["Main"]);
    store.set_owned_count("Main", CardVersionId(1), 5).unwrap();
    store.set_owned_count("Main", CardVersionId(2), 3).unwrap();
    store.dirty = false;

    let new_counts = HashMap::from([(CardVersionId(1), 10u32), (CardVersionId(3), 7)]);
    store.replace_profile_counts("Main", new_counts).unwrap();

    assert_eq!(store.owned_count("Main", CardVersionId(1)), 10);
    assert_eq!(store.owned_count("Main", CardVersionId(2)), 0); // removed
    assert_eq!(store.owned_count("Main", CardVersionId(3)), 7);
    assert!(store.needs_save());
}

#[test]
fn replace_counts_strips_zeros() {
    let mut store = store_with_profiles(&["Main"]);
    let counts = HashMap::from([(CardVersionId(1), 0u32), (CardVersionId(2), 5)]);
    store.replace_profile_counts("Main", counts).unwrap();

    let profile = store.profiles().iter().find(|p| p.name == "Main").unwrap();
    assert!(!profile.owned_counts.contains_key(&CardVersionId(1)));
    assert_eq!(profile.owned_counts[&CardVersionId(2)], 5);
}

#[test]
fn replace_counts_nonexistent_fails() {
    let mut store = store_with_profiles(&["Main"]);
    let err = store
        .replace_profile_counts("Ghost", HashMap::new())
        .unwrap_err();
    assert!(matches!(err, ProfileStoreError::NotFound(_)));
}

// --- load / save round-trip ---
// These tests require tokio, which is only a dev-dependency on non-wasm targets.

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_empty_storage_returns_new_store() {
    let store = ProfileStore::load(MemStorage::default()).await.unwrap();
    assert!(store.is_first_run());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn save_then_load_round_trip() {
    let storage = MemStorage::default();
    let mut store = ProfileStore::new(storage.clone());
    store.create_profile("Main".to_string()).unwrap();
    store.set_owned_count("Main", CardVersionId(7), 4).unwrap();
    store.save().await.unwrap();
    assert!(!store.needs_save());

    let loaded = ProfileStore::load(storage).await.unwrap();
    assert_eq!(loaded.owned_count("Main", CardVersionId(7)), 4);
    assert_eq!(loaded.primary_profile_name(), "Main");
    assert_eq!(loaded.active_profile_names(), &["Main"]);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn load_heals_empty_active_profile_names() {
    use crate::save_data::PROFILES_FORMAT_VERSION;
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
