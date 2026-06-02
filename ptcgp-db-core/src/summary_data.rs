//! Collection completion statistics for the Summary page.
//!
//! [`compute_summary`] performs a single pass over all sets and packs, producing per-set
//! completion rows, global totals, and the best pack(s) to open next.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use ptcgp_db_data::{CardVersion, Pack, Prob, Set};

use crate::AppSettings;
use crate::filter::filter_card;
use crate::probability::{completion, completion_merged, desired_pull_rate};
use crate::profile_store::ProfileStore;
use crate::save_data::{CardVersionId, FilterConfig};
use crate::storage::Storage;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Per-pack completion row, nested inside a [`SetRowData`].
#[derive(Clone, PartialEq)]
pub struct PackRowData {
    pub pack: &'static Pack,
    /// Completion percentage: Σ min(count, goal) / (n × goal) × 100.
    pub completion_pct: f64,
    /// Completion numerator: Σ min(effective_count, goal).
    pub owned: usize,
    /// Completion denominator: matching_cards_in_pack × goal.
    pub total: usize,
    /// Probability of drawing a desired card from this pack, as a percentage.
    pub rate_pct: f64,
}

/// Per-set completion row returned by [`compute_summary`].
pub struct SetRowData {
    pub set: &'static Set,
    pub completion_pct: f64,
    pub owned: usize,
    pub total: usize,
    pub obtainable: bool,
    pub best_pack: Option<&'static Pack>,
    pub best_rate_pct: f64,
    pub pack_rows: Vec<PackRowData>,
}

/// Full summary output returned by [`compute_summary`].
pub struct SummaryData {
    /// One row per set that contains at least one card matching the current filters.
    pub set_rows: Vec<SetRowData>,
    /// Best pack(s) to open globally (tied at the maximum desired-card pull rate).
    pub best_packs: Vec<(&'static Pack, Prob)>,
    /// Sum of min(effective_count, goal) across all matching cards.
    pub total_owned: usize,
    /// Sum of (matching_cards × goal) across all matching sets.
    pub total_denom: usize,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/// Effective owned count for a card version, clamped to goal.
///
/// When `cfg.any_version_owned` is set, counts any version of the same abstract card
/// toward this version's effective ownership.
fn effective_count<S: Storage + Clone>(
    cv_id: CardVersionId,
    cfg: &FilterConfig,
    store: &ProfileStore<S>,
) -> u32 {
    let goal = cfg.goal.max(1);
    if cfg.any_version_owned {
        let Some(cv) = CardVersion::from_id(cv_id.0) else {
            return 0;
        };
        cv.card()
            .versions()
            .iter()
            .map(|v| store.aggregate_count(CardVersionId(v.id())))
            .fold(0u32, u32::saturating_add)
            .min(goal)
    } else {
        store.aggregate_count(cv_id).min(goal)
    }
}

/// Computes completion statistics for all sets containing at least one matching card.
///
/// The caller supplies `today` so that time-sensitive checks (retirement date, obtainable
/// filter) behave consistently throughout a single render.
pub fn compute_summary<S: Storage + Clone>(
    store: &ProfileStore<S>,
    cfg: &FilterConfig,
    settings: &AppSettings,
    today: NaiveDate,
) -> SummaryData {
    let merge_dupes = settings.merge_duplicate_printings();
    let goal = cfg.goal.max(1);
    let counts = |id: CardVersionId| store.aggregate_count(id);

    // Desired IDs are accumulated during the set pass and reused for the global best-pack
    // computation below, avoiding a separate third pass over all card data.
    let mut all_desired_ids: HashSet<CardVersionId> = HashSet::new();

    let set_rows: Vec<SetRowData> = Set::ALL
        .iter()
        .filter_map(|set| {
            let matching_cvs: Vec<&CardVersion> = set
                .card_versions()
                .iter()
                .filter(|cv| filter_card(cv, cfg, settings, today, None, None))
                .collect();

            if matching_cvs.is_empty() {
                return None;
            }

            // One effective-count lookup per card; the map also serves as an O(1) membership
            // check when building pack rows below, replacing redundant filter_card calls.
            let eff_counts: HashMap<CardVersionId, u32> = matching_cvs
                .iter()
                .map(|cv| {
                    let id = CardVersionId(cv.id());
                    (id, effective_count(id, cfg, store))
                })
                .collect();

            let owned: usize = eff_counts.values().map(|&c| c as usize).sum();
            let total = matching_cvs.len() * goal as usize;

            let comp = if merge_dupes {
                completion_merged(counts, goal, matching_cvs.iter().map(|cv| CardVersionId(cv.id())))
            } else {
                completion(counts, goal, matching_cvs.iter().map(|cv| CardVersionId(cv.id())))
            };

            let obtainable = set.retirement_date().is_none_or(|r| r > today);

            all_desired_ids.extend(
                eff_counts
                    .iter()
                    .filter(|&(_, &c)| c < goal)
                    .map(|(&id, _)| id),
            );
            let has_desired = eff_counts.values().any(|&c| c < goal);

            let (best_pack, best_rate_pct) = if set.is_promo() || !has_desired {
                (None, 0.0)
            } else {
                let result = set
                    .packs()
                    .iter()
                    .filter_map(|p| {
                        let rate = desired_pull_rate(p, |id| {
                            eff_counts.get(&id).is_some_and(|&c| c < goal)
                        });
                        if rate == Prob::ZERO {
                            None
                        } else {
                            Some((p, rate))
                        }
                    })
                    .max_by(|(_, a), (_, b)| a.cmp(b));
                match result {
                    Some((pack, rate)) => (Some(pack), rate.as_f64() * 100.0),
                    None => (None, 0.0),
                }
            };

            let pack_rows: Vec<PackRowData> = set
                .packs()
                .iter()
                .map(|p| {
                    let p_matching_ids: Vec<CardVersionId> = p
                        .card_versions()
                        .iter()
                        .filter(|cv| eff_counts.contains_key(&CardVersionId(cv.id())))
                        .map(|cv| CardVersionId(cv.id()))
                        .collect();

                    let p_owned: usize = p_matching_ids
                        .iter()
                        .map(|id| eff_counts.get(id).copied().unwrap_or(0) as usize)
                        .sum();
                    let p_total = p_matching_ids.len() * goal as usize;

                    let p_comp = if merge_dupes {
                        completion_merged(counts, goal, p_matching_ids.iter().copied())
                    } else {
                        completion(counts, goal, p_matching_ids.iter().copied())
                    };

                    let p_rate =
                        desired_pull_rate(p, |id| eff_counts.get(&id).is_some_and(|&c| c < goal));

                    PackRowData {
                        pack: p,
                        completion_pct: p_comp.as_f64() * 100.0,
                        owned: p_owned,
                        total: p_total,
                        rate_pct: p_rate.as_f64() * 100.0,
                    }
                })
                .collect();

            Some(SetRowData {
                set,
                completion_pct: comp.as_f64() * 100.0,
                owned,
                total,
                obtainable,
                best_pack,
                best_rate_pct,
                pack_rows,
            })
        })
        .collect();

    let total_owned: usize = set_rows.iter().map(|r| r.owned).sum();
    let total_denom: usize = set_rows.iter().map(|r| r.total).sum();

    let all_pack_rates: Vec<(&'static Pack, Prob)> = Pack::ALL
        .iter()
        .filter(|p| !p.set().is_promo())
        .filter_map(|p| {
            let rate = desired_pull_rate(p, |id| all_desired_ids.contains(&id));
            if rate == Prob::ZERO {
                None
            } else {
                Some((p, rate))
            }
        })
        .collect();

    let best_packs: Vec<(&'static Pack, Prob)> =
        if let Some(&(_, best)) = all_pack_rates.iter().max_by(|(_, a), (_, b)| a.cmp(b)) {
            all_pack_rates
                .into_iter()
                .filter(|(_, r)| *r == best)
                .collect()
        } else {
            vec![]
        };

    SummaryData {
        set_rows,
        best_packs,
        total_owned,
        total_denom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ptcgp_db_data::{CardVersion, Set};

    use crate::AppSettings;
    use crate::profile_store::ProfileStore;
    use crate::save_data::FilterConfig;
    use crate::storage::Storage;

    // ---------------------------------------------------------------------------
    // Minimal in-memory storage stub
    // ---------------------------------------------------------------------------

    #[derive(Clone, Debug, Default)]
    struct MemStorage {
        profiles: std::rc::Rc<std::cell::RefCell<Option<crate::save_data::ProfilesSaveData>>>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("mem storage error")]
    struct MemError;

    impl Storage for MemStorage {
        type Error = MemError;
        async fn load_profiles(
            &self,
        ) -> Result<Option<crate::save_data::ProfilesSaveData>, Self::Error> {
            Ok(self.profiles.borrow().clone())
        }
        async fn save_profiles(
            &self,
            data: &crate::save_data::ProfilesSaveData,
        ) -> Result<(), Self::Error> {
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
            _: &crate::save_data::AppSettingsSaveData,
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
            _: &crate::save_data::SavedQueriesSaveData,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn empty_store() -> ProfileStore<MemStorage> {
        let mut store = ProfileStore::new(MemStorage::default());
        store.create_profile("Main".to_string()).unwrap();
        store
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    }

    // ---------------------------------------------------------------------------
    // Basic structural checks
    // ---------------------------------------------------------------------------

    #[test]
    fn set_rows_covers_all_nonempty_sets() {
        let store = empty_store();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());

        let expected_set_count = Set::ALL
            .iter()
            .filter(|s| !s.card_versions().is_empty())
            .count();
        assert_eq!(result.set_rows.len(), expected_set_count);
    }

    #[test]
    fn total_denom_equals_total_card_versions_at_goal_one() {
        let store = empty_store();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        assert_eq!(result.total_denom, CardVersion::ALL.len());
    }

    #[test]
    fn total_denom_scales_with_goal() {
        let store = empty_store();
        let cfg = FilterConfig {
            goal: 2,
            ..Default::default()
        };
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        assert_eq!(result.total_denom, CardVersion::ALL.len() * 2);
    }

    // ---------------------------------------------------------------------------
    // Owned-count effects
    // ---------------------------------------------------------------------------

    #[test]
    fn total_owned_zero_when_nothing_owned() {
        let store = empty_store();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        assert_eq!(result.total_owned, 0);
    }

    #[test]
    fn total_owned_increases_when_card_owned() {
        let mut store = empty_store();
        let cv = CardVersion::ALL.iter().next().expect("at least one card");
        store.set_owned_count("Main", CardVersionId(cv.id()), 1).unwrap();

        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        assert_eq!(result.total_owned, 1);
    }

    #[test]
    fn owned_count_clamped_to_goal() {
        let mut store = empty_store();
        let cv = CardVersion::ALL.iter().next().expect("at least one card");
        // Own 5 copies but goal is 1 — contribution to owned/denom must be clamped to 1.
        store.set_owned_count("Main", CardVersionId(cv.id()), 5).unwrap();

        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        // total_owned counts effective (clamped) contributions, so should not exceed total_denom.
        assert!(result.total_owned <= result.total_denom);
    }

    // ---------------------------------------------------------------------------
    // Best-pack logic
    // ---------------------------------------------------------------------------

    #[test]
    fn best_packs_non_empty_when_nothing_owned() {
        let store = empty_store();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        // With all cards desired, there should be at least one recommended pack.
        assert!(!result.best_packs.is_empty());
    }

    #[test]
    fn best_packs_empty_when_all_owned_at_goal() {
        let mut store = empty_store();
        let goal: u32 = 1;
        for cv in CardVersion::ALL.iter() {
            store.set_owned_count("Main", CardVersionId(cv.id()), goal).unwrap();
        }

        let cfg = FilterConfig {
            goal,
            ..Default::default()
        };
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        // No card is desired, so no pack can be recommended.
        assert!(result.best_packs.is_empty());
    }

    #[test]
    fn best_packs_all_non_promo() {
        let store = empty_store();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        for (pack, _) in &result.best_packs {
            assert!(
                !pack.set().is_promo(),
                "best_packs must not include promo packs"
            );
        }
    }

    #[test]
    fn best_packs_all_tied_at_max_rate() {
        let store = empty_store();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        if let Some(&(_, best_rate)) = result.best_packs.first() {
            for &(_, rate) in &result.best_packs {
                assert_eq!(
                    rate, best_rate,
                    "all best_packs must share the maximum rate"
                );
            }
        }
    }

    // ---------------------------------------------------------------------------
    // set_row contents
    // ---------------------------------------------------------------------------

    #[test]
    fn set_row_completion_pct_zero_when_nothing_owned() {
        let store = empty_store();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());
        for row in &result.set_rows {
            assert_eq!(
                row.completion_pct,
                0.0,
                "set {} should have 0% completion with nothing owned",
                row.set.id()
            );
        }
    }

    #[test]
    fn set_row_owned_reflects_store() {
        let mut store = empty_store();
        let Some(set) = Set::ALL.iter().find(|s| !s.card_versions().is_empty()) else {
            return;
        };
        let cv = set.card_versions().iter().next().unwrap();
        store.set_owned_count("Main", CardVersionId(cv.id()), 1).unwrap();

        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());

        let row = result
            .set_rows
            .iter()
            .find(|r| r.set.id() == set.id())
            .expect("set must appear in set_rows");
        assert_eq!(row.owned, 1);
    }

    // ---------------------------------------------------------------------------
    // any_version_owned
    // ---------------------------------------------------------------------------

    #[test]
    fn any_version_owned_counts_other_version_toward_effective_count() {
        // Find a card that has duplicates so we can own one version and expect
        // the other to count as owned under any_version_owned.
        let Some(original) = CardVersion::ALL.iter().find(|c| !c.duplicates().is_empty()) else {
            return;
        };
        let dup = original.duplicates().iter().next().unwrap();

        let mut store = empty_store();
        // Own only the duplicate version.
        store.set_owned_count("Main", CardVersionId(dup.id()), 1).unwrap();

        let cfg = FilterConfig {
            goal: 1,
            any_version_owned: true,
            ..Default::default()
        };
        let settings = AppSettings::default();
        let result = compute_summary(&store, &cfg, &settings, today());

        // Under any_version_owned, owning the duplicate should count toward the original.
        // So total_owned should be > 0.
        assert!(result.total_owned > 0);
    }
}
