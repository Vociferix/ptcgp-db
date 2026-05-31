//! Probability calculation engine for pack pull rates and collection completion.
//!
//! All intermediate arithmetic uses [`Prob`] (exact rational arithmetic). Convert to [`f64`]
//! or a percentage string only at final display time via [`Prob::as_f64`].
//!
//! Promo packs have pull-rate slot data and are handled correctly by all functions. Callers
//! that need to exclude promo packs for display or ranking purposes should filter on
//! `pack.set().is_promo()` themselves.

use std::collections::HashSet;

use ptcgp_db_data::{CardVersion, Pack, Prob};

use crate::save_data::CardVersionId;

/// Computes the probability ([0, 1]) that opening a specific pack will yield a specific card.
///
/// Uses the following formula:
/// ```text
/// P = Σ_v [ (1 - ∏_s [ 1 - card_rate(card, s) ]) × v.pull_rate ]
/// ```
/// where `card_rate(card, s)` is the `Prob` from the `CardVersionPullRate` entry for the card
/// in slot `s`, or zero if the card does not appear in that slot.
///
/// The inner product `∏_s (1 - card_rate(s))` is the probability the card appears in *no* slot
/// of variant `v`; subtracting from 1 gives P(card in at least one slot | variant v). Slots
/// within a variant are independent draws, so the product is exact.
///
/// **Performance note**: the innermost loop performs a linear scan over
/// `PackSlot::card_versions()` (hundreds of entries per slot) for each of the ~2–4 variants
/// and ~5–6 slots. For a single lookup this is negligible. A future optimization would be to
/// store a per-card, per-pack precomputed rate directly in `ptcgp-db-data`, reducing each
/// call to O(1). Profile before committing to that change.
pub fn card_pull_rate(pack: &Pack, card_id: CardVersionId) -> Prob {
    let mut total = Prob::ZERO;
    for variant in pack.variants() {
        let mut not_prob = Prob::ONE;
        for slot in variant.slots() {
            for cvpr in slot.card_versions() {
                if cvpr.card_version().id() == card_id {
                    not_prob *= Prob::ONE.saturating_sub(&cvpr.pull_rate());
                    break;
                }
            }
        }
        total =
            (total + (Prob::ONE.saturating_sub(&not_prob) * variant.pull_rate())).min(Prob::ONE);
    }
    total
}

/// Computes the probability ([0, 1]) that opening a specific pack will yield at least one
/// desired card.
///
/// Applies the following formula:
/// ```text
/// P = Σ_v [ (1 - ∏_s [ 1 - Σ_{desired c in s} c.pull_rate ]) × v.pull_rate ]
/// ```
///
/// Within a single slot the desired-card rates sum to P(any desired card in that slot), since
/// cards within a slot are mutually exclusive. The product `∏_s` accumulates the probability
/// that *no* slot yields a desired card; subtracting from 1 gives P(at least one desired card |
/// variant v). Results are in [0, 1].
///
/// Returns [`Prob::ZERO`] when `is_desired` returns `false` for every card, or when the pack
/// has no slot data (e.g. promo packs).
pub fn desired_pull_rate(pack: &Pack, is_desired: impl Fn(CardVersionId) -> bool) -> Prob {
    let mut total = Prob::ZERO;
    for variant in pack.variants() {
        let mut not_prob = Prob::ONE;
        for slot in variant.slots() {
            let mut slot_sum = Prob::ZERO;
            for cvpr in slot.card_versions() {
                if is_desired(cvpr.card_version().id()) {
                    slot_sum += cvpr.pull_rate();
                }
            }
            not_prob *= Prob::ONE.saturating_sub(&slot_sum);
        }
        total =
            (total + (Prob::ONE.saturating_sub(&not_prob) * variant.pull_rate())).min(Prob::ONE);
    }
    total
}

/// Returns the highest pull rate for a card version across all non-promo packs.
///
/// Iterates only the packs associated with this card version via `CardVersion::packs()` —
/// all other packs have an implicit rate of zero for this card.
pub fn max_card_pull_rate(card_id: CardVersionId) -> Prob {
    let Some(card) = CardVersion::from_id(card_id) else {
        return Prob::ZERO;
    };

    card.packs()
        .iter()
        .filter(|p| !p.set().is_promo())
        .map(|p| card_pull_rate(p, card_id))
        .max()
        .unwrap_or(Prob::ZERO)
}

/// Returns the non-promo pack with the highest desired pull rate, along with that rate.
///
/// Iterates all non-promo packs and applies [`desired_pull_rate`] with the given predicate.
/// Returns `None` only if every non-promo pack yields zero probability for the desired set
/// (i.e. no desired card appears in any non-promo pack).
pub fn best_pack_for_desired(
    is_desired: impl Fn(CardVersionId) -> bool,
) -> Option<(&'static Pack, Prob)> {
    Pack::ALL
        .iter()
        .filter(|p| !p.set().is_promo())
        .filter_map(|p| {
            let prob = desired_pull_rate(p, &is_desired);
            if prob == Prob::ZERO {
                None
            } else {
                Some((p, prob))
            }
        })
        .max_by(|(_, a), (_, b)| a.cmp(b))
}

/// Computes the completion fraction for a query set of card versions against owned counts.
///
/// Formula from DESIGN.md §Completion Formula:
/// ```text
/// numerator   = Σ min(count(c), T)   for all c in query
/// denominator = |query| × T
/// result      = numerator / denominator
/// ```
///
/// Returns [`Prob::ONE`] if `query` is empty (vacuously complete) and [`Prob::ZERO`] if
/// `target` is 0.
pub fn completion(
    counts: impl Fn(CardVersionId) -> u32,
    target: u32,
    query: impl IntoIterator<Item = CardVersionId>,
) -> Prob {
    if target == 0 {
        return Prob::ZERO;
    }

    let mut numerator: u64 = 0;
    let mut denominator: u64 = 0;

    for card_id in query {
        numerator += counts(card_id).min(target) as u64;
        denominator += target as u64;
    }

    if denominator == 0 {
        return Prob::ONE;
    }

    Prob::new(numerator, denominator)
}

/// Computes the completion fraction with "Merge duplicate printings" enabled.
///
/// Each duplicate group is treated as a single logical card. The count for a group is the
/// sum of owned counts across **all** versions in the group (not just those in the query),
/// clamped to `target`. Each group contributes exactly one entry to the denominator regardless
/// of how many versions fall within the query.
///
/// The group representative is the version where [`CardVersion::is_original`] is `true`. If
/// a card has no duplicates it forms a group of one.
///
/// Returns [`Prob::ONE`] if `query` is empty, [`Prob::ZERO`] if `target` is 0.
pub fn completion_merged(
    counts: impl Fn(CardVersionId) -> u32,
    target: u32,
    query: impl IntoIterator<Item = CardVersionId>,
) -> Prob {
    if target == 0 {
        return Prob::ZERO;
    }

    // Track group representatives already processed to avoid double-counting.
    let mut seen: HashSet<usize> = HashSet::new();
    let mut numerator: u64 = 0;
    let mut denominator: u64 = 0;

    for card_id in query {
        let Some(card) = CardVersion::from_id(card_id) else {
            continue;
        };

        // Identify the group by its original printing. If this card has no duplicates it is
        // its own representative.
        let rep_id = if card.is_original() {
            card_id
        } else {
            card.duplicates()
                .iter()
                .find(|d| d.is_original())
                .map(|d| d.id())
                .unwrap_or(card_id)
        };

        if !seen.insert(rep_id) {
            continue; // group already counted
        }

        // Sum owned counts across all versions in the duplicate group.
        let mut group_count: u32 = counts(card_id);
        for dup in card.duplicates().iter() {
            group_count = group_count.saturating_add(counts(dup.id()));
        }

        numerator += group_count.min(target) as u64;
        denominator += target as u64;
    }

    if denominator == 0 {
        return Prob::ONE;
    }

    Prob::new(numerator, denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ptcgp_db_data::CardVersion;

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    fn zero_counts(_: CardVersionId) -> u32 {
        0
    }

    fn one_counts(_: CardVersionId) -> u32 {
        1
    }

    fn two_counts(_: CardVersionId) -> u32 {
        2
    }

    fn count_for(owned_id: CardVersionId, count: u32) -> impl Fn(CardVersionId) -> u32 {
        move |id| if id == owned_id { count } else { 0 }
    }

    // Returns the first non-promo pack, panicking if none exists.
    fn first_non_promo_pack() -> &'static Pack {
        Pack::ALL
            .iter()
            .find(|p| !p.set().is_promo())
            .expect("test data must contain at least one non-promo pack")
    }

    // Returns the first promo pack, panicking if none exists.
    fn first_promo_pack() -> &'static Pack {
        Pack::ALL
            .iter()
            .find(|p| p.set().is_promo())
            .expect("test data must contain at least one promo pack")
    }

    // ---------------------------------------------------------------------------
    // card_pull_rate
    // ---------------------------------------------------------------------------

    #[test]
    fn card_pull_rate_promo_pack_present_card_is_positive() {
        // Promo packs have real slot data; the function computes the correct rate.
        // Callers that want to suppress promo packs filter on pack.set().is_promo().
        let pack = first_promo_pack();
        let card_id = pack
            .card_versions()
            .iter()
            .next()
            .map(|c| c.id())
            .expect("promo pack has at least one card");
        assert!(card_pull_rate(pack, card_id) > Prob::ZERO);
    }

    #[test]
    fn card_pull_rate_absent_card_returns_zero() {
        let pack = first_non_promo_pack();
        // CardVersionId::MAX is not a valid card — its rate must be zero.
        assert_eq!(card_pull_rate(pack, usize::MAX), Prob::ZERO);
    }

    #[test]
    fn card_pull_rate_present_card_is_positive() {
        let pack = first_non_promo_pack();
        let card_id = pack
            .card_versions()
            .iter()
            .next()
            .map(|c| c.id())
            .expect("non-promo pack has at least one card");
        assert!(card_pull_rate(pack, card_id) > Prob::ZERO);
    }

    #[test]
    fn pull_data_logic_non_zero_for_pack_cards() {
        // Mirrors the pull_data() OnceLock init in catalog.rs.
        let mut found_non_zero = false;
        for cv in CardVersion::ALL.iter() {
            let best = cv
                .packs()
                .iter()
                .filter(|p| !p.set().is_promo())
                .filter_map(|p| {
                    let rate = card_pull_rate(p, cv.id());
                    if rate > Prob::ZERO {
                        Some(rate.as_f64() * 100.0)
                    } else {
                        None
                    }
                })
                .reduce(f64::max);
            if let Some(pct) = best {
                assert!(
                    pct > 0.0,
                    "cv {} has non-ZERO rate but as_f64()*100 = {}",
                    cv.id(),
                    pct
                );
                found_non_zero = true;
                break;
            }
        }
        assert!(
            found_non_zero,
            "no pack card had a non-zero rate — pull_data logic is broken"
        );
    }

    // ---------------------------------------------------------------------------
    // desired_pull_rate
    // ---------------------------------------------------------------------------

    #[test]
    fn desired_pull_rate_none_desired_returns_zero() {
        let pack = first_non_promo_pack();
        assert_eq!(desired_pull_rate(pack, |_| false), Prob::ZERO);
    }

    #[test]
    fn desired_pull_rate_promo_pack_all_desired_is_positive() {
        // Promo packs have real slot data; the function computes the correct rate.
        // Callers that want to suppress promo packs filter on pack.set().is_promo().
        let pack = first_promo_pack();
        assert!(desired_pull_rate(pack, |_| true) > Prob::ZERO);
    }

    #[test]
    fn desired_pull_rate_all_desired_equals_one() {
        // With every card desired, each slot's desired-card rates sum to exactly 1.0
        // (since all card rates in a slot sum to 1.0). The "not" probability for each
        // slot is then 1 - 1 = 0, the product across slots is 0, and P(at least one
        // desired) = 1 - 0 = 1.0 for every variant. Weighted sum across variants also
        // equals 1.0.
        let pack = first_non_promo_pack();
        assert_eq!(desired_pull_rate(pack, |_| true), Prob::ONE);
    }

    #[test]
    fn desired_pull_rate_single_card_matches_card_pull_rate() {
        let pack = first_non_promo_pack();
        let card_id = pack
            .card_versions()
            .iter()
            .next()
            .map(|c| c.id())
            .expect("non-promo pack has at least one card");

        let via_desired = desired_pull_rate(pack, |id| id == card_id);
        let via_card = card_pull_rate(pack, card_id);
        assert_eq!(via_desired, via_card);
    }

    // ---------------------------------------------------------------------------
    // max_card_pull_rate
    // ---------------------------------------------------------------------------

    #[test]
    fn max_card_pull_rate_invalid_id_returns_zero() {
        assert_eq!(max_card_pull_rate(usize::MAX), Prob::ZERO);
    }

    #[test]
    fn max_card_pull_rate_pack_card_is_positive() {
        // Find a card version whose source is "Pack" (has non-empty packs()).
        let card_id = CardVersion::ALL
            .iter()
            .find(|c| !c.packs().is_empty() && !c.set().is_promo())
            .map(|c| c.id())
            .expect("data must contain at least one non-promo pack card");
        assert!(max_card_pull_rate(card_id) > Prob::ZERO);
    }

    #[test]
    fn max_card_pull_rate_non_pack_card_returns_zero() {
        // Find a card version with no packs (non-Pack source).
        let result = CardVersion::ALL.iter().find(|c| c.packs().is_empty());
        if let Some(card) = result {
            assert_eq!(max_card_pull_rate(card.id()), Prob::ZERO);
        }
        // If no such card exists in the test data, the test vacuously passes.
    }

    // ---------------------------------------------------------------------------
    // best_pack_for_desired
    // ---------------------------------------------------------------------------

    #[test]
    fn best_pack_for_desired_none_desired_returns_none() {
        assert!(best_pack_for_desired(|_| false).is_none());
    }

    #[test]
    fn best_pack_for_desired_all_desired_returns_some() {
        let result = best_pack_for_desired(|_| true);
        assert!(result.is_some());
        let (pack, prob) = result.unwrap();
        assert!(!pack.set().is_promo());
        assert!(prob > Prob::ZERO);
    }

    #[test]
    fn best_pack_for_desired_result_is_maximum() {
        // Verify the returned rate is at least as large as every other non-promo pack's rate.
        let (_, best_prob) = best_pack_for_desired(|_| true).unwrap();
        for pack in Pack::ALL.iter().filter(|p| !p.set().is_promo()) {
            let rate = desired_pull_rate(pack, |_| true);
            assert!(
                best_prob >= rate,
                "best_prob {best_prob:#} < pack rate {rate:#}"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // completion
    // ---------------------------------------------------------------------------

    #[test]
    fn completion_target_zero_returns_zero() {
        assert_eq!(completion(one_counts, 0, [0usize]), Prob::ZERO);
    }

    #[test]
    fn completion_empty_query_returns_one() {
        assert_eq!(completion(zero_counts, 1, []), Prob::ONE);
    }

    #[test]
    fn completion_none_owned_t1_returns_zero() {
        let ids: Vec<usize> = CardVersion::ALL.iter().take(10).map(|c| c.id()).collect();
        assert_eq!(completion(zero_counts, 1, ids), Prob::ZERO);
    }

    #[test]
    fn completion_all_owned_t1_returns_one() {
        let ids: Vec<usize> = (0..5).collect();
        assert_eq!(completion(one_counts, 1, ids), Prob::ONE);
    }

    #[test]
    fn completion_half_owned_t1() {
        // 5 cards, only card 0 owned
        let ids: Vec<usize> = (0..5).collect();
        let counts = count_for(0, 1);
        let result = completion(counts, 1, ids);
        assert_eq!(result, Prob::new(1, 5));
    }

    #[test]
    fn completion_t2_count_one() {
        // 1 card, count=1, T=2 → 1/2
        assert_eq!(completion(one_counts, 2, [0usize]), Prob::new(1, 2));
    }

    #[test]
    fn completion_t2_count_two_returns_one() {
        assert_eq!(completion(two_counts, 2, [0usize]), Prob::ONE);
    }

    #[test]
    fn completion_count_exceeds_target_clamped() {
        // count=5, T=2 → clamped to min(5,2)=2 → 2/2 = 1
        let high = |_: CardVersionId| 5u32;
        assert_eq!(completion(high, 2, [0usize]), Prob::ONE);
    }

    #[test]
    fn completion_two_cards_one_owned_t2() {
        // 2 cards, card 0 count=2, card 1 count=0, T=2 → (2+0)/(2×2) = 2/4 = 1/2
        let counts = count_for(0, 2);
        let result = completion(counts, 2, [0usize, 1]);
        assert_eq!(result, Prob::new(1, 2));
    }

    // ---------------------------------------------------------------------------
    // completion_merged
    // ---------------------------------------------------------------------------

    #[test]
    fn completion_merged_target_zero_returns_zero() {
        assert_eq!(completion_merged(one_counts, 0, [0usize]), Prob::ZERO);
    }

    #[test]
    fn completion_merged_empty_query_returns_one() {
        assert_eq!(completion_merged(zero_counts, 1, []), Prob::ONE);
    }

    #[test]
    fn completion_merged_no_duplicates_matches_completion() {
        // For cards without duplicates, merged and non-merged formulas should agree.
        let ids: Vec<usize> = CardVersion::ALL
            .iter()
            .take(20)
            .filter(|c| c.duplicates().is_empty())
            .map(|c| c.id())
            .collect();

        if ids.is_empty() {
            return; // vacuously pass if no such cards
        }

        let result_plain = completion(one_counts, 1, ids.iter().copied());
        let result_merged = completion_merged(one_counts, 1, ids.iter().copied());
        assert_eq!(result_plain, result_merged);
    }

    #[test]
    fn completion_merged_deduplicates_groups() {
        // Find a card version that has at least one duplicate.
        let Some(original) = CardVersion::ALL.iter().find(|c| !c.duplicates().is_empty()) else {
            return; // vacuously pass if no duplicates in test data
        };
        let dup = original.duplicates().iter().next().unwrap();

        // Query contains both the original and its duplicate — should count as one card.
        let ids = [original.id(), dup.id()];
        let result = completion_merged(zero_counts, 1, ids);
        // denominator = 1×T = 1, numerator = 0 → 0/1 = ZERO
        assert_eq!(result, Prob::ZERO);

        // With one owned copy the group is complete at T=1.
        let counts = count_for(original.id(), 1);
        let result = completion_merged(counts, 1, ids);
        assert_eq!(result, Prob::ONE);
    }

    #[test]
    fn completion_merged_sums_counts_across_group() {
        // Find a card with a duplicate.
        let Some(original) = CardVersion::ALL
            .iter()
            .find(|c| !c.duplicates().is_empty() && c.is_original())
        else {
            return;
        };
        let dup = original.duplicates().iter().next().unwrap();

        // Own 1 copy of original + 1 copy of duplicate → group_count=2, T=2 → complete.
        let counts = move |id: CardVersionId| {
            if id == original.id() || id == dup.id() {
                1
            } else {
                0
            }
        };
        // Query includes only the original; the merged formula should still see both copies.
        let result = completion_merged(counts, 2, [original.id()]);
        assert_eq!(
            result,
            Prob::ONE,
            "group count should sum across all versions"
        );
    }
}
