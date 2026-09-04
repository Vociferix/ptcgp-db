//! Pack-point purchase suggestions.
//!
//! [`build_purchases`] ranks the cards the user still needs by how much collection progress
//! each pack point buys.  Like the trade algorithms in [`crate::trade_data`], it is a pure
//! data-in / data-out function over `ProfileStore`.

use chrono::NaiveDate;
use ptcgp_db_data::{CardVersion, Prob};

use crate::AppSettings;
use crate::filter::filter_card;
use crate::probability::max_card_pull_rate;
use crate::profile_store::ProfileStore;
use crate::save_data::{CardVersionId, FilterConfig};
use crate::storage::Storage;
use crate::trade_data::raw_dest_count;

/// A card worth buying from a set's pack-point shop.
#[derive(Clone, PartialEq)]
pub struct PurchaseRec {
    pub cv: &'static CardVersion,
    /// Aggregate owned count across active profiles.
    pub dest_count: u32,
    /// Copies still needed to reach the goal.
    pub needed: u32,
    /// Highest aggregate pull rate across all non-promo packs.
    pub max_rate: Prob,
    /// Pack points required to buy one copy.
    pub cost: u32,
    /// Sort key: `cost × max_rate`. Lower is a better buy.
    pub value: f64,
}

/// Returns a ranked list of pack-point purchase suggestions, best value first.
///
/// A card qualifies when the aggregate count across active profiles is below the goal and the
/// card is obtainable from packs (`max_pull_rate > 0`).  Cards with no non-promo pack have no
/// set shop to buy them from and are omitted, as are cards from retired sets, whose shops can
/// no longer be reached.  `max_cost` caps the pack points per card so the list is not dominated
/// by cards the user cannot afford; `None` means no cap.
///
/// Ranking is ascending by `pack_point_cost × max_pull_rate`: a card scores well by being cheap,
/// by being hard to pull from packs, or by any balance of the two.
pub fn build_purchases<S: Storage + Clone>(
    store: &ProfileStore<S>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    matched_name_ids: Option<&[usize]>,
    max_cost: Option<u32>,
) -> Vec<PurchaseRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;
    let mut recs: Vec<PurchaseRec> = Vec::new();

    for cv in CardVersion::ALL {
        if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
            continue;
        }

        let cost = cv.rarity().pack_point_cost();
        if max_cost.is_some_and(|max| cost > max) {
            continue;
        }

        // The pack-point shop for a retired set is gone, so its cards cannot be bought
        // regardless of the Obtainable filter.
        if cv.set().retirement_date().is_some_and(|d| d <= today) {
            continue;
        }

        if !filter_card(cv, cfg, settings, today, matched_name_ids, None) {
            continue;
        }

        let raw = raw_dest_count(cv, store, merge_dupes, any_version);
        if raw >= goal {
            continue;
        }

        let max_rate = max_card_pull_rate(CardVersionId(cv.id()));
        if max_rate == Prob::ZERO {
            continue;
        }

        recs.push(PurchaseRec {
            cv,
            dest_count: raw,
            needed: goal - raw,
            max_rate,
            cost,
            value: cost as f64 * max_rate.as_f64(),
        });
    }

    recs.sort_by(|a, b| {
        a.value
            .partial_cmp(&b.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recs
}

/// Distinct pack-point costs across all rarities, ascending.
///
/// Costs come from a small fixed set of rarity tiers, so the max-cost filter offers exactly
/// these values rather than an arbitrary number — every threshold in between is equivalent to
/// the next one down.
pub fn pack_point_cost_tiers() -> Vec<u32> {
    let mut tiers: Vec<u32> = ptcgp_db_data::Rarity::ALL
        .iter()
        .map(|r| r.pack_point_cost())
        .collect();
    tiers.sort_unstable();
    tiers.dedup();
    tiers
}

#[cfg(test)]
mod tests {
    use super::*;
    use ptcgp_db_data::CardVersion;

    use crate::AppSettings;
    use crate::profile_store::ProfileStore;
    use crate::save_data::FilterConfig;
    use crate::storage::Storage;

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

    fn store() -> ProfileStore<MemStorage> {
        let mut store = ProfileStore::new(MemStorage::default());
        store.create_profile("Dest".to_string()).unwrap();
        store
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    }

    fn cfg(goal: u32) -> FilterConfig {
        FilterConfig {
            goal,
            ..Default::default()
        }
    }

    /// A buyable card: pack-obtainable, from a set that has not retired.
    fn buyable_card() -> Option<&'static CardVersion> {
        CardVersion::ALL.iter().find(|c| {
            !c.packs().is_empty()
                && !c.set().is_promo()
                && c.set().retirement_date().is_none_or(|d| d > today())
                && max_card_pull_rate(CardVersionId(c.id())) != Prob::ZERO
        })
    }

    #[test]
    fn owned_at_goal_is_not_suggested() {
        let Some(cv) = buyable_card() else { return };
        let mut store = store();
        store
            .set_owned_count("Dest", CardVersionId(cv.id()), 1)
            .unwrap();

        let recs = build_purchases(
            &store,
            &AppSettings::default(),
            &cfg(1),
            today(),
            None,
            None,
        );
        assert!(!recs.iter().any(|r| r.cv.id() == cv.id()));
    }

    #[test]
    fn needed_card_is_suggested_with_its_rarity_cost() {
        let Some(cv) = buyable_card() else { return };
        let recs = build_purchases(
            &store(),
            &AppSettings::default(),
            &cfg(1),
            today(),
            None,
            None,
        );
        let rec = recs
            .iter()
            .find(|r| r.cv.id() == cv.id())
            .expect("unowned pack card should be suggested");
        assert_eq!(rec.cost, cv.rarity().pack_point_cost());
        assert_eq!(rec.needed, 1);
    }

    #[test]
    fn cards_without_a_pack_pull_rate_are_omitted() {
        let recs = build_purchases(
            &store(),
            &AppSettings::default(),
            &cfg(1),
            today(),
            None,
            None,
        );
        assert!(recs.iter().all(|r| r.max_rate != Prob::ZERO));
    }

    #[test]
    fn max_cost_excludes_pricier_cards() {
        let tiers = pack_point_cost_tiers();
        let cap = tiers.first().copied().expect("at least one rarity tier");
        let recs = build_purchases(
            &store(),
            &AppSettings::default(),
            &cfg(1),
            today(),
            None,
            Some(cap),
        );
        assert!(recs.iter().all(|r| r.cost <= cap));
    }

    #[test]
    fn sorted_ascending_by_cost_times_pull_rate() {
        let recs = build_purchases(
            &store(),
            &AppSettings::default(),
            &cfg(1),
            today(),
            None,
            None,
        );
        assert!(
            recs.windows(2).all(|w| w[0].value <= w[1].value),
            "purchase suggestions must be ordered best-value first"
        );
    }

    #[test]
    fn cost_tiers_are_sorted_and_deduplicated() {
        let tiers = pack_point_cost_tiers();
        assert!(tiers.windows(2).all(|w| w[0] < w[1]));
    }
}
