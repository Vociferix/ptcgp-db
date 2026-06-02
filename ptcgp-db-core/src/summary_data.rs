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
        let Some(cv) = CardVersion::from_id(cv_id) else {
            return 0;
        };
        cv.card()
            .versions()
            .iter()
            .map(|v| store.aggregate_count(v.id()))
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
                .map(|cv| (cv.id(), effective_count(cv.id(), cfg, store)))
                .collect();

            let owned: usize = eff_counts.values().map(|&c| c as usize).sum();
            let total = matching_cvs.len() * goal as usize;

            let comp = if merge_dupes {
                completion_merged(counts, goal, matching_cvs.iter().map(|cv| cv.id()))
            } else {
                completion(counts, goal, matching_cvs.iter().map(|cv| cv.id()))
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
                        .filter(|cv| eff_counts.contains_key(&cv.id()))
                        .map(|cv| cv.id())
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
