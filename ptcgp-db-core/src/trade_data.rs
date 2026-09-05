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
use crate::save_data::{CardVersionId, FilterConfig};
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
    /// `true` when the source still needs `card_a` (its count is below the goal), making it the
    /// more attractive offer.  `false` marks a fallback pick: the source already has enough of
    /// the card, but it is still the cheapest thing the destination can part with.
    pub card_a_source_wants: bool,
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
            .map(|v| store.aggregate_count(CardVersionId(v.id())))
            .fold(0u32, u32::saturating_add)
    } else if merge_dupes {
        let mut total = store.aggregate_count(CardVersionId(cv.id()));
        for dup in cv.duplicates() {
            total = total.saturating_add(store.aggregate_count(CardVersionId(dup.id())));
        }
        total
    } else {
        store.aggregate_count(CardVersionId(cv.id()))
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
        let mut total = store.owned_count(profile_name, CardVersionId(cv.id()));
        for dup in cv.duplicates() {
            total = total.saturating_add(store.owned_count(profile_name, CardVersionId(dup.id())));
        }
        total
    } else {
        store.owned_count(profile_name, CardVersionId(cv.id()))
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

        let max_rate = max_card_pull_rate(CardVersionId(cv.id()));
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
///
/// `card_a` prefers a card the source still needs, falling back to one it already has when no
/// wanted card is available — see [`TradeRec::card_a_source_wants`].
pub fn build_trades<S: Storage + Clone>(
    store: &ProfileStore<S>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    inactive_names: &[String],
    matched_name_ids: Option<&[usize]>,
) -> Vec<TradeRec> {
    let goal = cfg.goal.max(1);
    let excess_threshold = cfg.trade_excess_threshold.max(goal);
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
            max_rate: max_card_pull_rate(CardVersionId(cv.id())),
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

            // Cards the source still wants are the better offer, but a card it already has is
            // far better than no recommendation at all: at the rarer classes the destination
            // often has excess of only one card, so requiring `src_cnt < goal` here would drop
            // the whole (source, rarity class) pair. Rank wanted cards first and fall back.
            let best_a = card_data
                .iter()
                .zip(src_counts.iter())
                .filter(|(d, _)| {
                    d.rarity_class_id == rarity_class_id
                        && d.dest_raw > excess_threshold
                        && d.max_rate != Prob::ZERO
                })
                .min_by(|(da, sa), (db, sb)| {
                    let wants_a = **sa < goal;
                    let wants_b = **sb < goal;
                    wants_b.cmp(&wants_a).then_with(|| {
                        let va =
                            1.0 / (da.max_rate.as_f64() * (da.dest_raw - excess_threshold) as f64);
                        let vb =
                            1.0 / (db.max_rate.as_f64() * (db.dest_raw - excess_threshold) as f64);
                        va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                    })
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
                card_a_source_wants: a_src_count < goal,
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
/// Only tradable cards with a non-zero pull rate are included.
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
        if !cv.is_tradable() {
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

        let max_rate = max_card_pull_rate(CardVersionId(cv.id()));
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
    use ptcgp_db_data::{CardVersion, RarityClass};

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
        store
            .set_owned_count("Dest", CardVersionId(cv.id()), 3)
            .unwrap();
        store
            .set_owned_count("Source", CardVersionId(cv.id()), 7)
            .unwrap();
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
        store
            .set_owned_count("Dest", CardVersionId(original.id()), 2)
            .unwrap();
        store
            .set_owned_count("Dest", CardVersionId(dup.id()), 3)
            .unwrap();

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
        store
            .set_owned_count("Dest", CardVersionId(cv.id()), 1)
            .unwrap();
        store
            .set_owned_count("Dest", CardVersionId(other_ver.id()), 2)
            .unwrap();

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
        store
            .set_owned_count("Source", CardVersionId(cv.id()), 5)
            .unwrap();
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
        store
            .set_owned_count("Source", CardVersionId(original.id()), 2)
            .unwrap();
        store
            .set_owned_count("Source", CardVersionId(dup.id()), 4)
            .unwrap();

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
            store
                .set_owned_count("Dest", CardVersionId(cv.id()), goal)
                .unwrap();
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
        store
            .set_owned_count("Source", CardVersionId(cv.id()), 1)
            .unwrap();

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

    /// Regression: a source that already owns the destination's only excess card in a rarity
    /// class used to suppress that class entirely, so no trade was ever offered for it.
    #[test]
    fn source_owning_the_excess_card_still_yields_a_trade() {
        let Some(class_id) = star_one_class_id() else {
            return;
        };
        let pool: Vec<&'static CardVersion> = CardVersion::ALL
            .iter()
            .filter(|c| c.is_tradable() && c.rarity().class().id() == class_id)
            .collect();
        if pool.len() < 5 {
            return;
        }
        let give = pool[0];

        let mut store = store_with_two_profiles();
        // Dest holds 3 copies of `give` — its only excess card in this class.
        store
            .set_owned_count("Dest", CardVersionId(give.id()), 3)
            .unwrap();
        // Source holds other cards of the same class that Dest needs, and one copy of `give`.
        for want in &pool[1..5] {
            store
                .set_owned_count("Source", CardVersionId(want.id()), 1)
                .unwrap();
        }
        store
            .set_owned_count("Source", CardVersionId(give.id()), 1)
            .unwrap();

        let cfg = FilterConfig {
            goal: 1,
            trade_excess_threshold: 2,
            ..Default::default()
        };
        let inactive = vec!["Source".to_string()];
        let recs = build_trades(
            &store,
            &AppSettings::default(),
            &cfg,
            today(),
            &inactive,
            None,
        );

        let rec = recs
            .iter()
            .find(|r| r.card_a.rarity().class().id() == class_id)
            .expect("a trade should still be offered when the source already owns the give card");
        assert_eq!(rec.card_a.id(), give.id());
        assert!(
            !rec.card_a_source_wants,
            "the fallback pick must be flagged as one the source already has"
        );
    }

    /// A card the source still needs outranks one it already has.
    #[test]
    fn card_a_prefers_a_card_the_source_still_wants() {
        let Some(class_id) = star_one_class_id() else {
            return;
        };
        let pool: Vec<&'static CardVersion> = CardVersion::ALL
            .iter()
            .filter(|c| c.is_tradable() && c.rarity().class().id() == class_id)
            .collect();
        if pool.len() < 6 {
            return;
        }
        let owned_by_source = pool[0];
        let wanted_by_source = pool[1];

        let mut store = store_with_two_profiles();
        store
            .set_owned_count("Dest", CardVersionId(owned_by_source.id()), 5)
            .unwrap();
        store
            .set_owned_count("Dest", CardVersionId(wanted_by_source.id()), 3)
            .unwrap();
        store
            .set_owned_count("Source", CardVersionId(owned_by_source.id()), 1)
            .unwrap();
        for want in &pool[2..6] {
            store
                .set_owned_count("Source", CardVersionId(want.id()), 1)
                .unwrap();
        }

        let cfg = FilterConfig {
            goal: 1,
            trade_excess_threshold: 2,
            ..Default::default()
        };
        let inactive = vec!["Source".to_string()];
        let recs = build_trades(
            &store,
            &AppSettings::default(),
            &cfg,
            today(),
            &inactive,
            None,
        );

        let rec = recs
            .iter()
            .find(|r| r.card_a.rarity().class().id() == class_id)
            .expect("a trade should be offered");
        assert_eq!(
            rec.card_a.id(),
            wanted_by_source.id(),
            "a card the source still needs must outrank one it already has"
        );
        assert!(rec.card_a_source_wants);
    }

    /// Deterministic xorshift, so a failing case is reproducible without a dev-dependency.
    fn xorshift(state: &mut u64) -> u64 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *state = x;
        x
    }

    /// Independent oracle: does any valid (card_a, card_b) pair exist for this rarity class?
    fn trade_pair_exists(
        store: &ProfileStore<MemStorage>,
        settings: &AppSettings,
        cfg: &FilterConfig,
        source: &str,
        class_id: usize,
    ) -> bool {
        let goal = cfg.goal.max(1);
        let threshold = cfg.trade_excess_threshold.max(goal);
        let merge = settings.merge_duplicate_printings();
        let any_version = cfg.any_version_owned;
        let (mut has_a, mut has_b) = (false, false);

        for cv in CardVersion::ALL {
            if merge && !cv.is_original() && !cv.duplicates().is_empty() {
                continue;
            }
            if !cv.is_tradable()
                || !filter_card(cv, cfg, settings, today(), None, None)
                || cv.rarity().class().id() != class_id
            {
                continue;
            }
            let dest = raw_dest_count(cv, store, merge, any_version);
            let src = raw_source_count(cv, store, source, merge);
            if dest < goal && src > 0 {
                has_b = true;
            }
            if dest > threshold && max_card_pull_rate(CardVersionId(cv.id())) != Prob::ZERO {
                has_a = true;
            }
        }
        has_a && has_b
    }

    /// Whenever a rarity class has both a card to receive and a card to give, `build_trades`
    /// must list a recommendation for it. This is the invariant the Card A fallback restores.
    #[test]
    fn every_class_with_an_available_pair_is_listed() {
        let inactive = vec!["Source".to_string()];
        let mut seed = 0x2026_0904_u64;

        for goal in 1..=3u32 {
            for threshold in 1..=4u32 {
                for merge in [false, true] {
                    for any_version in [false, true] {
                        let mut settings = AppSettings::default();
                        settings.set_merge_duplicate_printings(merge);
                        let cfg = FilterConfig {
                            goal,
                            trade_excess_threshold: threshold,
                            any_version_owned: any_version,
                            ..Default::default()
                        };

                        let mut store = store_with_two_profiles();
                        for _ in 0..250 {
                            let idx = (xorshift(&mut seed) as usize) % CardVersion::ALL.len();
                            let cv = &CardVersion::ALL[idx];
                            let dest_n = (xorshift(&mut seed) % 6) as u32;
                            let src_n = (xorshift(&mut seed) % 3) as u32;
                            if dest_n > 0 {
                                store
                                    .set_owned_count("Dest", CardVersionId(cv.id()), dest_n)
                                    .unwrap();
                            }
                            if src_n > 0 {
                                store
                                    .set_owned_count("Source", CardVersionId(cv.id()), src_n)
                                    .unwrap();
                            }
                        }

                        let recs = build_trades(&store, &settings, &cfg, today(), &inactive, None);
                        for class_id in 0..RarityClass::ALL.len() {
                            let expected =
                                trade_pair_exists(&store, &settings, &cfg, "Source", class_id);
                            let listed = recs
                                .iter()
                                .any(|r| r.card_a.rarity().class().id() == class_id);
                            assert_eq!(
                                expected, listed,
                                "goal={goal} keep={threshold} merge={merge} \
                                 any_version={any_version} class={class_id}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The 1-star (Star, one symbol) class the reported bug was found in.
    fn star_one_class_id() -> Option<usize> {
        RarityClass::ALL
            .iter()
            .find(|c| c.group().name().as_str() == "Star" && c.count() == 1)
            .map(|c| c.id())
    }

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
        store
            .set_owned_count("Dest", CardVersionId(cv.id()), 2)
            .unwrap();

        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let result = build_candidates(&store, &settings, &cfg, today(), None, true);
        assert!(result.iter().any(|r| r.cv.id() == cv.id()));
    }

    #[test]
    fn build_candidates_excludes_non_tradable_cards() {
        let mut store = store_with_two_profiles();
        // Find a card version that is not tradable but has a non-zero pull rate (pack card).
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| !c.is_tradable() && !c.packs().is_empty())
        else {
            return; // no such card in data set — skip
        };
        // Own excess copies.
        store
            .set_owned_count("Dest", CardVersionId(cv.id()), 5)
            .unwrap();

        let settings = AppSettings::default();
        let cfg = FilterConfig {
            goal: 1,
            ..Default::default()
        };
        let result = build_candidates(&store, &settings, &cfg, today(), None, true);
        assert!(
            !result.iter().any(|r| r.cv.id() == cv.id()),
            "non-tradable card should not appear in candidates"
        );
    }

    #[test]
    fn build_candidates_sorted_by_scarcity_ascending() {
        // Verify the returned list is in ascending scarcity order (lowest 1/rate×excess first).
        let mut store = store_with_two_profiles();
        // Give Dest 2 copies of every pack card (goal=1 → excess=1 for all).
        for cv in CardVersion::ALL.iter().filter(|c| !c.packs().is_empty()) {
            store
                .set_owned_count("Dest", CardVersionId(cv.id()), 2)
                .unwrap();
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
