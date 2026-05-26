//! Saved filter configurations shared across all profiles.

use crate::{
    save_data::{FilterConfig, QUERIES_FORMAT_VERSION, SavedQueriesSaveData, SavedQuery},
    storage::Storage,
};

/// Named filter configurations shared across all profiles.
///
/// Create via [`SavedQueries::load`] on startup; use the provided methods to add, remove,
/// rename, and update queries. Call [`save`] to persist changes.
///
/// This type is provided as a Dioxus context at the app root (T09).
///
/// [`save`]: SavedQueries::save
#[derive(Clone, Debug, PartialEq)]
pub struct SavedQueries {
    data: SavedQueriesSaveData,
}

impl SavedQueries {
    /// Constructs a `SavedQueries` from deserialized save data.
    pub fn from_save_data(data: SavedQueriesSaveData) -> Self {
        Self { data }
    }

    /// Returns a reference to the underlying save data for persistence.
    pub fn as_save_data(&self) -> &SavedQueriesSaveData {
        &self.data
    }

    /// Loads saved queries from the storage backend, returning an empty list when none exist.
    pub async fn load<S: Storage>(storage: &S) -> Result<Self, S::Error> {
        match storage.load_saved_queries().await? {
            Some(data) => Ok(Self::from_save_data(data)),
            None => Ok(Self::default()),
        }
    }

    /// Persists the current saved queries to the storage backend.
    pub async fn save<S: Storage>(&self, storage: &S) -> Result<(), S::Error> {
        storage.save_saved_queries(&self.data).await
    }

    /// All saved queries in the order they were created.
    pub fn queries(&self) -> &[SavedQuery] {
        &self.data.queries
    }

    /// Adds a new named query. Returns `false` without modifying state if a query with the
    /// same name already exists.
    pub fn add(&mut self, name: String, config: FilterConfig) -> bool {
        if self.data.queries.iter().any(|q| q.name == name) {
            return false;
        }
        self.data.queries.push(SavedQuery { name, config });
        true
    }

    /// Removes the query with the given name. Returns `false` if no such query exists.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.data.queries.len();
        self.data.queries.retain(|q| q.name != name);
        self.data.queries.len() < before
    }

    /// Updates the filter config of an existing query by name. Returns `false` if no query
    /// with that name exists.
    pub fn update(&mut self, name: &str, config: FilterConfig) -> bool {
        if let Some(q) = self.data.queries.iter_mut().find(|q| q.name == name) {
            q.config = config;
            true
        } else {
            false
        }
    }

    /// Renames an existing query. Returns `false` if `old_name` does not exist or if
    /// `new_name` is already taken by a different query.
    pub fn rename(&mut self, old_name: &str, new_name: String) -> bool {
        if self.data.queries.iter().any(|q| q.name == new_name) {
            return false;
        }
        if let Some(q) = self.data.queries.iter_mut().find(|q| q.name == old_name) {
            q.name = new_name;
            true
        } else {
            false
        }
    }
}

impl Default for SavedQueries {
    fn default() -> Self {
        Self {
            data: SavedQueriesSaveData {
                format_version: QUERIES_FORMAT_VERSION,
                queries: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(goal: u32) -> FilterConfig {
        FilterConfig {
            goal,
            ..FilterConfig::default()
        }
    }

    #[test]
    fn default_is_empty() {
        assert!(SavedQueries::default().queries().is_empty());
    }

    #[test]
    fn add_and_retrieve() {
        let mut sq = SavedQueries::default();
        assert!(sq.add("Diamond Run".to_string(), make_config(1)));
        assert_eq!(sq.queries().len(), 1);
        assert_eq!(sq.queries()[0].name, "Diamond Run");
        assert_eq!(sq.queries()[0].config.goal, 1);
    }

    #[test]
    fn add_duplicate_name_rejected() {
        let mut sq = SavedQueries::default();
        assert!(sq.add("Q".to_string(), make_config(1)));
        assert!(!sq.add("Q".to_string(), make_config(2)));
        assert_eq!(sq.queries().len(), 1);
    }

    #[test]
    fn remove_existing() {
        let mut sq = SavedQueries::default();
        sq.add("A".to_string(), make_config(1));
        sq.add("B".to_string(), make_config(2));
        assert!(sq.remove("A"));
        assert_eq!(sq.queries().len(), 1);
        assert_eq!(sq.queries()[0].name, "B");
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let mut sq = SavedQueries::default();
        assert!(!sq.remove("nope"));
    }

    #[test]
    fn update_existing() {
        let mut sq = SavedQueries::default();
        sq.add("Q".to_string(), make_config(1));
        assert!(sq.update("Q", make_config(2)));
        assert_eq!(sq.queries()[0].config.goal, 2);
    }

    #[test]
    fn update_nonexistent_returns_false() {
        let mut sq = SavedQueries::default();
        assert!(!sq.update("nope", make_config(1)));
    }

    #[test]
    fn rename_existing() {
        let mut sq = SavedQueries::default();
        sq.add("Old".to_string(), make_config(1));
        assert!(sq.rename("Old", "New".to_string()));
        assert_eq!(sq.queries()[0].name, "New");
    }

    #[test]
    fn rename_collision_rejected() {
        let mut sq = SavedQueries::default();
        sq.add("A".to_string(), make_config(1));
        sq.add("B".to_string(), make_config(2));
        assert!(!sq.rename("A", "B".to_string()));
        assert_eq!(sq.queries()[0].name, "A");
    }

    #[test]
    fn rename_nonexistent_returns_false() {
        let mut sq = SavedQueries::default();
        assert!(!sq.rename("nope", "new".to_string()));
    }

    #[test]
    fn round_trip_save_data() {
        let mut sq = SavedQueries::default();
        sq.add("Q1".to_string(), make_config(1));
        sq.add("Q2".to_string(), make_config(2));

        let data = sq.as_save_data().clone();
        let sq2 = SavedQueries::from_save_data(data);
        assert_eq!(sq, sq2);
    }
}
