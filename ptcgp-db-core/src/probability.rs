//! Probability calculation engine for pack pull rates and collection completion.
//!
//! All intermediate arithmetic uses [`Prob`] (exact rational arithmetic). Convert to [`f64`]
//! or a percentage string only at final display time via [`Prob::as_f64`].
//!
//! **Promo packs are excluded from all calculations in this module.** Functions that operate
//! on a specific [`Pack`] return [`Prob::ZERO`] when given a promo pack.

use std::collections::HashSet;

use ptcgp_db_data::{CardVersion, Pack, Prob};

use crate::save_data::CardVersionId;

/// Computes the aggregate pull probability for a single card version from a specific pack.
///
/// Uses the formula from DESIGN.md §Per-Card Pull Rate for a Pack:
/// ```text
/// P = Σ_v [ v.pull_rate × Σ_s card_rate(card, s) ]
/// ```
/// where `card_rate(card, s)` is the `Prob` from the `CardVersionPullRate` entry for the card
/// in slot `s`, or zero if the card does not appear in that slot.
///
/// Returns [`Prob::ZERO`] for promo packs.
pub fn card_pull_rate(pack: &Pack, card_id: CardVersionId) -> Prob {
    if pack.set().is_promo() {
        return Prob::ZERO;
    }

    let mut total = Prob::ZERO;
    for variant in pack.variants() {
        let mut slot_sum = Prob::ZERO;
        for slot in variant.slots() {
            for cvpr in slot.card_versions() {
                if cvpr.card_version().id() == card_id {
                    slot_sum += cvpr.pull_rate();
                    break; // each card appears at most once per slot
                }
            }
        }
        total += variant.pull_rate() * slot_sum;
    }
    total
}

/// Computes the expected number of "desired" cards yielded per opening of a specific pack.
///
/// Uses the formula from DESIGN.md §Probability of Pulling Any "Desired" Card:
/// ```text
/// P = Σ_v [ v.pull_rate × Σ_s Σ_{desired c in s} c.pull_rate ]
/// ```
/// Within any given slot, cards are mutually exclusive (only one card appears per slot), so
/// individual desired-card rates sum correctly. Across multiple slots the result may exceed 1
/// when a pack yields many cards with high individual rates.
///
/// Returns [`Prob::ZERO`] for promo packs or when `is_desired` returns `false` for every card.
pub fn desired_pull_rate(pack: &Pack, is_desired: impl Fn(CardVersionId) -> bool) -> Prob {
    if pack.set().is_promo() {
        return Prob::ZERO;
    }

    let mut total = Prob::ZERO;
    for variant in pack.variants() {
        let mut slot_sum = Prob::ZERO;
        for slot in variant.slots() {
            for cvpr in slot.card_versions() {
                if is_desired(cvpr.card_version().id()) {
                    slot_sum += cvpr.pull_rate();
                }
            }
        }
        total += variant.pull_rate() * slot_sum;
    }
    total
}

/// Returns the highest pull rate for a card version across all non-promo packs.
///
/// Iterates only the packs associated with this card version via `CardVersion::packs()` —
/// all other packs have an implicit rate of zero for this card.
///
/// Returns [`Prob::ZERO`] if the card has no non-promo packs (promo cards or cards with a
/// non-Pack source).
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
    fn card_pull_rate_promo_pack_returns_zero() {
        let pack = first_promo_pack();
        let card_id = pack
            .card_versions()
            .iter()
            .next()
            .map(|c| c.id())
            .expect("promo pack has at least one card");
        assert_eq!(card_pull_rate(pack, card_id), Prob::ZERO);
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

    // ---------------------------------------------------------------------------
    // desired_pull_rate
    // ---------------------------------------------------------------------------

    #[test]
    fn desired_pull_rate_none_desired_returns_zero() {
        let pack = first_non_promo_pack();
        assert_eq!(desired_pull_rate(pack, |_| false), Prob::ZERO);
    }

    #[test]
    fn desired_pull_rate_promo_pack_returns_zero() {
        let pack = first_promo_pack();
        assert_eq!(desired_pull_rate(pack, |_| true), Prob::ZERO);
    }

    #[test]
    fn desired_pull_rate_all_desired_is_positive() {
        let pack = first_non_promo_pack();
        assert!(desired_pull_rate(pack, |_| true) > Prob::ZERO);
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
