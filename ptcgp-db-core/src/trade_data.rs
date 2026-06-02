//! Trade recommendation algorithms and supporting data types.
//!
//! [`build_shares`], [`build_trades`], and [`build_candidates`] implement the three trade-page
//! recommendation algorithms.  Each is a pure data-in / data-out function: it reads from
//! `ProfileStore` and returns a ranked `Vec` of recommendation records.

use std::cmp::Reverse;
use std::collections::HashSet;

use chrono::NaiveDate;
use ptcgp_db_data::{CardVersion, Prob};

use crate::AppSettings;
use crate::filter::filter_card;
use crate::probability::max_card_pull_rate;
use crate::profile_store::ProfileStore;
use crate::save_data::FilterConfig;
use crate::storage::Storage;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A profile that holds excess copies of a card.
#[derive(Clone, PartialEq)]
pub struct SourceInfo {
    /// Profile name.
    pub name: String,
    /// Aggregate owned count in this profile.
    pub count: u32,
}

/// A recommended one-sided share: a card the destination needs, held by a source profile.
#[derive(Clone, PartialEq)]
pub struct ShareRec {
    pub cv: &'static CardVersion,
    pub dest_count: u32,
    pub needed: u32,
    pub max_rate: Prob,
    /// `true` when `max_pull_rate == 0` — card is unobtainable from packs (top-priority tier).
    pub is_zero_rate: bool,
    pub best_source: SourceInfo,
    pub alt_sources: Vec<SourceInfo>,
}

/// A recommended two-sided trade between a source profile and the destination.
#[derive(Clone, PartialEq)]
pub struct TradeRec {
    pub source_name: String,
    /// Card the destination receives from the source.
    pub card_b: &'static CardVersion,
    pub card_b_dest_count: u32,
    pub card_b_source_count: u32,
    pub card_b_max_rate: Prob,
    pub card_b_receive_value: f64,
    /// Card the destination gives to the source.
    pub card_a: &'static CardVersion,
    pub card_a_dest_count: u32,
    pub card_a_source_count: u32,
    pub card_a_max_rate: Prob,
}

/// A card the destination holds in excess — a good candidate to give in trades.
#[derive(Clone, PartialEq)]
pub struct CandidateRec {
    pub cv: &'static CardVersion,
    pub dest_count: u32,
    pub excess: u32,
    pub max_rate: Prob,
    pub is_unobtainable: bool,
}

// ---------------------------------------------------------------------------
// Count helpers (pub for reuse; used internally by all three algorithms)
// ---------------------------------------------------------------------------

/// Aggregate destination count with optional merge-duplicates / any-version semantics.
pub fn raw_dest_count<S: Storage + Clone>(
    cv: &CardVersion,
    store: &ProfileStore<S>,
    merge_dupes: bool,
    any_version: bool,
) -> u32 {
    if any_version {
        cv.card()
            .versions()
            .iter()
            .map(|v| store.aggregate_count(v.id()))
            .fold(0u32, u32::saturating_add)
    } else if merge_dupes {
        let mut total = store.aggregate_count(cv.id());
        for dup in cv.duplicates() {
            total = total.saturating_add(store.aggregate_count(dup.id()));
        }
        total
    } else {
        store.aggregate_count(cv.id())
    }
}

/// Owned count for a specific named profile, with optional merge-duplicates.
pub fn raw_source_count<S: Storage + Clone>(
    cv: &CardVersion,
    store: &ProfileStore<S>,
    profile_name: &str,
    merge_dupes: bool,
) -> u32 {
    if merge_dupes {
        let mut total = store.owned_count(profile_name, cv.id());
        for dup in cv.duplicates() {
            total = total.saturating_add(store.owned_count(profile_name, dup.id()));
        }
        total
    } else {
        store.owned_count(profile_name, cv.id())
    }
}

// ---------------------------------------------------------------------------
// Algorithms
// ---------------------------------------------------------------------------

/// Returns a ranked list of one-sided share recommendations.
///
/// Only Diamond-rarity, tradable cards are considered.  Cards are ranked by
/// pull-rate scarcity (zero-rate cards first, then by 1 / (rate × needed)).
pub fn build_shares<S: Storage + Clone>(
    store: &ProfileStore<S>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    inactive_names: &[String],
    matched_name_ids: Option<&[usize]>,
) -> Vec<ShareRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;
    let mut recs: Vec<ShareRec> = Vec::new();

    for cv in CardVersion::ALL {
        if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
            continue;
        }
        if !cv.is_tradable() {
            continue;
        }
        if cv.rarity().group().name().as_str() != "Diamond" {
            continue;
        }
        if !filter_card(cv, cfg, settings, today, matched_name_ids, None) {
            continue;
        }

        let raw = raw_dest_count(cv, store, merge_dupes, any_version);
        if raw >= goal {
            continue;
        }
        let needed = goal - raw;

        let mut sources: Vec<SourceInfo> = inactive_names
            .iter()
            .filter_map(|name| {
                let cnt = raw_source_count(cv, store, name, merge_dupes);
                if cnt > 0 {
                    Some(SourceInfo {
                        name: name.clone(),
                        count: cnt,
                    })
                } else {
                    None
                }
            })
            .collect();

        if sources.is_empty() {
            continue;
        }

        sources.sort_by_key(|s| Reverse(s.count));
        let best_source = sources.remove(0);
        let alt_sources = sources;

        let max_rate = max_card_pull_rate(cv.id());
        let is_zero_rate = max_rate == Prob::ZERO;

        recs.push(ShareRec {
            cv,
            dest_count: raw,
            needed,
            max_rate,
            is_zero_rate,
            best_source,
            alt_sources,
        });
    }

    recs.sort_by(|a, b| match (a.is_zero_rate, b.is_zero_rate) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (true, true) => a.needed.cmp(&b.needed),
        (false, false) => {
            let va = 1.0 / (a.max_rate.as_f64() * a.needed as f64);
            let vb = 1.0 / (b.max_rate.as_f64() * b.needed as f64);
            vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    recs
}

/// Returns a ranked list of two-sided trade recommendations.
///
/// For each source profile and rarity class, finds the best card the source can provide
/// (`card_b`) and the best card the destination can give back (`card_a`).  Pairs are ranked
/// by the receive value of `card_b`.
pub fn build_trades<S: Storage + Clone>(
    store: &ProfileStore<S>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    inactive_names: &[String],
    matched_name_ids: Option<&[usize]>,
) -> Vec<TradeRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;

    struct CardData {
        cv: &'static CardVersion,
        dest_raw: u32,
        rarity_class_id: usize,
        max_rate: Prob,
    }

    let card_data: Vec<CardData> = CardVersion::ALL
        .iter()
        .filter(|cv| {
            if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
                return false;
            }
            cv.is_tradable() && filter_card(cv, cfg, settings, today, matched_name_ids, None)
        })
        .map(|cv| CardData {
            cv,
            dest_raw: raw_dest_count(cv, store, merge_dupes, any_version),
            rarity_class_id: cv.rarity().class().id(),
            max_rate: max_card_pull_rate(cv.id()),
        })
        .collect();

    let mut recs: Vec<TradeRec> = Vec::new();

    for source_name in inactive_names {
        let src_counts: Vec<u32> = card_data
            .iter()
            .map(|d| raw_source_count(d.cv, store, source_name, merge_dupes))
            .collect();

        let rarity_class_ids: Vec<usize> = {
            let mut seen: HashSet<usize> = HashSet::new();
            card_data
                .iter()
                .map(|d| d.rarity_class_id)
                .filter(|&id| seen.insert(id))
                .collect()
        };

        for rarity_class_id in rarity_class_ids {
            let best_b = card_data
                .iter()
                .zip(src_counts.iter())
                .filter(|(d, src_cnt)| {
                    d.rarity_class_id == rarity_class_id && d.dest_raw < goal && **src_cnt > 0
                })
                .max_by(|(da, _), (db, _)| {
                    match (da.max_rate == Prob::ZERO, db.max_rate == Prob::ZERO) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => {
                            let va = if da.max_rate == Prob::ZERO {
                                f64::INFINITY
                            } else {
                                1.0 / (da.max_rate.as_f64() * (goal - da.dest_raw) as f64)
                            };
                            let vb = if db.max_rate == Prob::ZERO {
                                f64::INFINITY
                            } else {
                                1.0 / (db.max_rate.as_f64() * (goal - db.dest_raw) as f64)
                            };
                            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    }
                });

            let Some((b_data, b_src_count_ref)) = best_b else {
                continue;
            };
            let b_src_count = *b_src_count_ref;

            let b_receive_value = if b_data.max_rate == Prob::ZERO {
                f64::INFINITY
            } else {
                1.0 / (b_data.max_rate.as_f64() * (goal - b_data.dest_raw) as f64)
            };

            let best_a = card_data
                .iter()
                .zip(src_counts.iter())
                .filter(|(d, src_cnt)| {
                    d.rarity_class_id == rarity_class_id
                        && d.dest_raw > goal
                        && **src_cnt < goal
                        && d.max_rate != Prob::ZERO
                })
                .min_by(|(da, _), (db, _)| {
                    let va = 1.0 / (da.max_rate.as_f64() * (da.dest_raw - goal) as f64);
                    let vb = 1.0 / (db.max_rate.as_f64() * (db.dest_raw - goal) as f64);
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                });

            let Some((a_data, a_src_count_ref)) = best_a else {
                continue;
            };
            let a_src_count = *a_src_count_ref;

            recs.push(TradeRec {
                source_name: source_name.clone(),
                card_b: b_data.cv,
                card_b_dest_count: b_data.dest_raw,
                card_b_source_count: b_src_count,
                card_b_max_rate: b_data.max_rate,
                card_b_receive_value: b_receive_value,
                card_a: a_data.cv,
                card_a_dest_count: a_data.dest_raw,
                card_a_source_count: a_src_count,
                card_a_max_rate: a_data.max_rate,
            });
        }
    }

    recs.sort_by(|a, b| {
        b.card_b_receive_value
            .partial_cmp(&a.card_b_receive_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recs
}

/// Returns a ranked list of trade candidates: cards the destination holds in excess of `goal`.
///
/// Only cards with a non-zero pull rate are included (untradable / no-pack cards are excluded).
/// Retired-set cards are filtered by `show_unobtainable`.
pub fn build_candidates<S: Storage + Clone>(
    store: &ProfileStore<S>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    matched_name_ids: Option<&[usize]>,
    show_unobtainable: bool,
) -> Vec<CandidateRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;
    let mut recs: Vec<CandidateRec> = Vec::new();

    for cv in CardVersion::ALL {
        if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
            continue;
        }
        if !filter_card(cv, cfg, settings, today, matched_name_ids, None) {
            continue;
        }

        let raw = raw_dest_count(cv, store, merge_dupes, any_version);
        if raw <= goal {
            continue;
        }
        let excess = raw - goal;

        let max_rate = max_card_pull_rate(cv.id());
        if max_rate == Prob::ZERO {
            continue;
        }

        let is_unobtainable = cv.set().retirement_date().is_some_and(|d| d <= today);
        if is_unobtainable && !show_unobtainable {
            continue;
        }

        recs.push(CandidateRec {
            cv,
            dest_count: raw,
            excess,
            max_rate,
            is_unobtainable,
        });
    }

    recs.sort_by(|a, b| {
        let va = 1.0 / (a.max_rate.as_f64() * a.excess as f64);
        let vb = 1.0 / (b.max_rate.as_f64() * b.excess as f64);
        va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
    });
    recs
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ptcgp_db_data::CardVersion;

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

    fn store_with_two_profiles() -> ProfileStore<MemStorage> {
        let mut store = ProfileStore::new(MemStorage::default());
        store.create_profile("Dest".to_string()).unwrap();
        store.create_profile("Source".to_string()).unwrap();
        store
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    }

    // ---------------------------------------------------------------------------
    // raw_dest_count
    // ---------------------------------------------------------------------------

    #[test]
    fn raw_dest_count_zero_when_nothing_owned() {
        let store = store_with_two_profiles();
        let cv = CardVersion::ALL.iter().next().expect("at least one card");
        assert_eq!(raw_dest_count(cv, &store, false, false), 0);
    }

    #[test]
    fn raw_dest_count_reads_active_profile() {
        let mut store = store_with_two_profiles();
        let cv = CardVersion::ALL.iter().next().expect("at least one card");
        // "Dest" is the primary/active profile; "Source" is inactive.
        store.set_owned_count("Dest", cv.id(), 3).unwrap();
        store.set_owned_count("Source", cv.id(), 7).unwrap();
        // Only the active ("Dest") count should be reflected.
        assert_eq!(raw_dest_count(cv, &store, false, false), 3);
    }

    #[test]
    fn raw_dest_count_merge_dupes_sums_duplicates() {
        let Some(original) = CardVersion::ALL.iter().find(|c| !c.duplicates().is_empty()) else {
            return;
        };
        let dup = original.duplicates().iter().next().unwrap();
        let mut store = store_with_two_profiles();
        store.set_owned_count("Dest", original.id(), 2).unwrap();
        store.set_owned_count("Dest", dup.id(), 3).unwrap();

        // With merge_dupes=true the counts across duplicate versions should sum.
        assert_eq!(raw_dest_count(original, &store, true, false), 5);
    }

    #[test]
    fn raw_dest_count_any_version_sums_all_versions_of_card() {
        // Find a card version whose abstract card has more than one version.
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.card().versions().len() > 1)
        else {
            return;
        };
        let other_ver = cv
            .card()
            .versions()
            .iter()
            .find(|v| v.id() != cv.id())
            .unwrap();

        let mut store = store_with_two_profiles();
        store.set_owned_count("Dest", cv.id(), 1).unwrap();
        store.set_owned_count("Dest", other_ver.id(), 2).unwrap();

        assert_eq!(raw_dest_count(cv, &store, false, true), 3);
    }

    // ---------------------------------------------------------------------------
    // raw_source_count
    // ---------------------------------------------------------------------------

    #[test]
    fn raw_source_count_zero_when_nothing_owned() {
        let store = store_with_two_profiles();
        let cv = CardVersion::ALL.iter().next().expect("at least one card");
        assert_eq!(raw_source_count(cv, &store, "Source", false), 0);
    }

    #[test]
    fn raw_source_count_reads_named_profile() {
        let mut store = store_with_two_profiles();
        let cv = CardVersion::ALL.iter().next().expect("at least one card");
        store.set_owned_count("Source", cv.id(), 5).unwrap();
        assert_eq!(raw_source_count(cv, &store, "Source", false), 5);
        // Active profile ("Dest") should not bleed into source count.
        assert_eq!(raw_source_count(cv, &store, "Dest", false), 0);
    }

    #[test]
    fn raw_source_count_merge_dupes_sums_duplicates() {
        let Some(original) = CardVersion::ALL.iter().find(|c| !c.duplicates().is_empty()) else {
            return;
        };
        let dup = original.duplicates().iter().next().unwrap();
        let mut store = store_with_two_profiles();
        store.set_owned_count("Source", original.id(), 2).unwrap();
        store.set_owned_count("Source", dup.id(), 4).unwrap();

        assert_eq!(raw_source_count(original, &store, "Source", true), 6);
    }

    // ---------------------------------------------------------------------------
    // build_shares — smoke tests
    // ---------------------------------------------------------------------------

    #[test]
    fn build_shares_empty_inactive_names_returns_empty() {
        let store = store_with_two_profiles();
        let settings = AppSettings::default();
        let cfg = FilterConfig::default();
        let result = build_shares(&store, &settings, &cfg, today(), &[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_shares_all_dest_owned_returns_empty() {
        let mut store = store_with_two_profiles();
        let goal: u32 = 1;
        for cv in CardVersion::ALL.iter() {
            store.set_owned_count("Dest", cv.id(), goal).unwrap();
        }
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal,
            ..Default::default()
        };
        let inactive = vec!["Source".to_string()];
        let result = build_shares(&store, &settings, &cfg, today(), &inactive, None);
        // Nothing is needed, so no shares should be returned.
        assert!(result.is_empty());
    }

    #[test]
    fn build_shares_source_has_card_dest_needs_returns_recommendation() {
        let mut store = store_with_two_profiles();
        // Find a tradable diamond-rarity card version.
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.is_tradable() && c.rarity().group().name().as_str() == "Diamond")
        else {
            return;
        };
        // Source has the card; Dest does not.
        store.set_owned_count("Source", cv.id(), 1).unwrap();

        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let inactive = vec!["Source".to_string()];
        let result = build_shares(&store, &settings, &cfg, today(), &inactive, None);
        assert!(!result.is_empty());
        // The first rec should reference our cv (or at least some rec should).
        assert!(result.iter().any(|r| r.cv.id() == cv.id()));
    }

    // ---------------------------------------------------------------------------
    // build_trades — smoke tests
    // ---------------------------------------------------------------------------

    #[test]
    fn build_trades_empty_inactive_names_returns_empty() {
        let store = store_with_two_profiles();
        let settings = AppSettings::default();
        let cfg = FilterConfig::default();
        let result = build_trades(&store, &settings, &cfg, today(), &[], None);
        assert!(result.is_empty());
    }

    // ---------------------------------------------------------------------------
    // build_candidates — smoke tests
    // ---------------------------------------------------------------------------

    #[test]
    fn build_candidates_no_excess_returns_empty() {
        let store = store_with_two_profiles();
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let result = build_candidates(&store, &settings, &cfg, today(), None, true);
        assert!(result.is_empty());
    }

    #[test]
    fn build_candidates_excess_pack_card_is_included() {
        let mut store = store_with_two_profiles();
        // Find a non-promo pack card (has a non-zero max pull rate).
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| !c.packs().is_empty() && !c.set().is_promo())
        else {
            return;
        };
        // Own 2 copies with goal=1 → excess=1.
        store.set_owned_count("Dest", cv.id(), 2).unwrap();

        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let result = build_candidates(&store, &settings, &cfg, today(), None, true);
        assert!(result.iter().any(|r| r.cv.id() == cv.id()));
    }

    #[test]
    fn build_candidates_sorted_by_scarcity_ascending() {
        // Verify the returned list is in ascending scarcity order (lowest 1/rate×excess first).
        let mut store = store_with_two_profiles();
        // Give Dest 2 copies of every pack card (goal=1 → excess=1 for all).
        for cv in CardVersion::ALL.iter().filter(|c| !c.packs().is_empty()) {
            store.set_owned_count("Dest", cv.id(), 2).unwrap();
        }

        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let result = build_candidates(&store, &settings, &cfg, today(), None, true);

        // Each consecutive pair should have non-decreasing scarcity value.
        for window in result.windows(2) {
            let va = 1.0 / (window[0].max_rate.as_f64() * window[0].excess as f64);
            let vb = 1.0 / (window[1].max_rate.as_f64() * window[1].excess as f64);
            assert!(
                va <= vb + f64::EPSILON,
                "build_candidates not sorted: {va} > {vb}"
            );
        }
    }
}
