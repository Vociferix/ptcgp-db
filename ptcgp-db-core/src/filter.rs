//! Card-version filter predicate shared across Catalog, Summary, and Trade pages.

use chrono::NaiveDate;
use ptcgp_db_data::CardVersion;

use crate::AppSettings;
use crate::save_data::{CardKindFilter, CountThreshold, FilterConfig};

/// Returns `true` when `cv` satisfies all active filter criteria.
///
/// `name_ids` is the pre-resolved set of card-name IDs matching the current name query; pass
/// `None` to skip the name filter (i.e. when the query is empty or absent).
///
/// `owned_count` is the caller-supplied aggregate owned count for this card version; pass
/// `Some(count)` to enable the `cfg.owned_count` threshold check (Catalog page only).  Pass
/// `None` to skip that check — Summary and Trade pages omit the owned-count threshold.
pub fn filter_card(
    cv: &CardVersion,
    cfg: &FilterConfig,
    settings: &AppSettings,
    today: NaiveDate,
    name_ids: Option<&[usize]>,
    owned_count: Option<u32>,
) -> bool {
    if settings.ignore_unobtainable_sets() && cv.set().retirement_date().is_some_and(|d| d <= today)
    {
        return false;
    }
    if settings.ignore_premium_mission() && cv.source().name().as_str() == "Premium Mission" {
        return false;
    }
    if settings.ignore_gold_shop() && cv.source().name().as_str() == "Gold Shop" {
        return false;
    }

    if name_ids.is_some_and(|ids| !ids.contains(&cv.card().name().id())) {
        return false;
    }

    if cfg.series.is_some_and(|sid| cv.series().id() != sid) {
        return false;
    }
    if !cfg.sets.is_empty() && !cfg.sets.contains(&cv.set().id()) {
        return false;
    }
    if !cfg.packs.is_empty() && !cv.packs().iter().any(|p| cfg.packs.contains(&p.id())) {
        return false;
    }
    if !cfg.rarities.is_empty() && !cfg.rarities.contains(&cv.rarity().class().id()) {
        return false;
    }

    match cfg.card_kind {
        Some(CardKindFilter::Pokemon) if !cv.card().is_pokemon() => return false,
        Some(CardKindFilter::Trainer) if !cv.card().is_trainer() => return false,
        _ => {}
    }

    let pkmn = cv.card().pokemon();
    if let Some(ex_only) = cfg.ex
        && pkmn.is_none_or(|p| p.is_ex() != ex_only)
    {
        return false;
    }
    if let Some(mega_only) = cfg.mega
        && pkmn.is_none_or(|p| p.is_mega() != mega_only)
    {
        return false;
    }
    if let Some(stage_id) = cfg.stage
        && pkmn.is_none_or(|p| p.stage().id() != stage_id)
    {
        return false;
    }
    if !cfg.elements.is_empty() && pkmn.is_none_or(|p| !cfg.elements.contains(&p.element().id())) {
        return false;
    }
    if cfg.foil.is_some_and(|f| cv.is_foil() != f) {
        return false;
    }
    if !cfg.sources.is_empty() && !cfg.sources.contains(&cv.source().id()) {
        return false;
    }
    if let Some(obtainable) = cfg.obtainable {
        let is_obtainable = cv.set().retirement_date().is_none_or(|d| d > today);
        if is_obtainable != obtainable {
            return false;
        }
    }
    if let Some(count) = owned_count
        && let Some(thresh) = cfg.owned_count
    {
        let ok = match thresh {
            CountThreshold::Equal(n) => count == n,
            CountThreshold::LessThan(n) => count < n,
            CountThreshold::AtLeast(n) => count >= n,
        };
        if !ok {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ptcgp_db_data::CardVersion;

    use crate::save_data::{CardKindFilter, CountThreshold, FilterConfig};
    use crate::settings::AppSettings;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
    }

    fn far_future() -> NaiveDate {
        NaiveDate::from_ymd_opt(2099, 1, 1).unwrap()
    }

    fn first_cv() -> &'static CardVersion {
        CardVersion::ALL
            .iter()
            .next()
            .expect("at least one card version")
    }

    // ---------------------------------------------------------------------------
    // Default config — everything passes
    // ---------------------------------------------------------------------------

    #[test]
    fn default_config_passes_all_cards() {
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        for cv in CardVersion::ALL.iter() {
            assert!(
                filter_card(cv, &cfg, &settings, today(), None, None),
                "default config should pass card {}",
                cv.id()
            );
        }
    }

    // ---------------------------------------------------------------------------
    // name_ids filter
    // ---------------------------------------------------------------------------

    #[test]
    fn name_filter_empty_ids_rejects_all() {
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        for cv in CardVersion::ALL.iter() {
            assert!(!filter_card(cv, &cfg, &settings, today(), Some(&[]), None));
        }
    }

    #[test]
    fn name_filter_matching_id_accepts() {
        let cv = first_cv();
        let name_id = cv.card().name().id();
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        assert!(filter_card(
            cv,
            &cfg,
            &settings,
            today(),
            Some(&[name_id]),
            None
        ));
    }

    #[test]
    fn name_filter_non_matching_id_rejects() {
        let cv = first_cv();
        let name_id = cv.card().name().id();
        // Find a different name ID that this card definitely doesn't have.
        let other_id = if name_id == 0 { 1 } else { 0 };
        let cfg = FilterConfig::default();
        let settings = AppSettings::default();
        // Only valid if other_id actually differs (it always does given name_id ∈ {0,1,...}).
        assert!(!filter_card(
            cv,
            &cfg,
            &settings,
            today(),
            Some(&[other_id]),
            None
        ));
    }

    // ---------------------------------------------------------------------------
    // Series filter
    // ---------------------------------------------------------------------------

    #[test]
    fn series_filter_matching_accepts() {
        let cv = first_cv();
        let cfg = FilterConfig {
            series: Some(cv.series().id()),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, None));
    }

    #[test]
    fn series_filter_non_matching_rejects() {
        let cv = first_cv();
        let sid = cv.series().id();
        // Find a card version from a different series to be sure the id is foreign.
        let Some(other_cv) = CardVersion::ALL.iter().find(|c| c.series().id() != sid) else {
            return; // only one series in test data — vacuously pass
        };
        let cfg = FilterConfig {
            series: Some(other_cv.series().id()),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    // ---------------------------------------------------------------------------
    // Set filter
    // ---------------------------------------------------------------------------

    #[test]
    fn set_filter_matching_accepts() {
        let cv = first_cv();
        let cfg = FilterConfig {
            sets: vec![cv.set().id()],
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, None));
    }

    #[test]
    fn set_filter_non_matching_rejects() {
        let cv = first_cv();
        let set_id = cv.set().id();
        let Some(other_cv) = CardVersion::ALL.iter().find(|c| c.set().id() != set_id) else {
            return;
        };
        let cfg = FilterConfig {
            sets: vec![other_cv.set().id()],
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    // ---------------------------------------------------------------------------
    // Pack filter
    // ---------------------------------------------------------------------------

    #[test]
    fn pack_filter_matching_accepts() {
        let Some(cv) = CardVersion::ALL.iter().find(|c| !c.packs().is_empty()) else {
            return;
        };
        let cfg = FilterConfig {
            packs: vec![cv.packs()[0].id()],
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, None));
    }

    #[test]
    fn pack_filter_non_matching_rejects() {
        use ptcgp_db_data::Pack;
        // Find a card with packs, then filter on a pack it doesn't belong to.
        let Some(cv) = CardVersion::ALL.iter().find(|c| !c.packs().is_empty()) else {
            return;
        };
        let cv_pack_ids: Vec<usize> = cv.packs().iter().map(|p| p.id()).collect();
        let Some(other_pack) = Pack::ALL.iter().find(|p| !cv_pack_ids.contains(&p.id())) else {
            return;
        };
        let cfg = FilterConfig {
            packs: vec![other_pack.id()],
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    // ---------------------------------------------------------------------------
    // Rarity filter
    // ---------------------------------------------------------------------------

    #[test]
    fn rarity_filter_matching_accepts() {
        let cv = first_cv();
        let cfg = FilterConfig {
            rarities: vec![cv.rarity().class().id()],
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, None));
    }

    #[test]
    fn rarity_filter_non_matching_rejects() {
        let cv = first_cv();
        let rarity_id = cv.rarity().class().id();
        let Some(other_cv) = CardVersion::ALL
            .iter()
            .find(|c| c.rarity().class().id() != rarity_id)
        else {
            return;
        };
        let cfg = FilterConfig {
            rarities: vec![other_cv.rarity().class().id()],
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    // ---------------------------------------------------------------------------
    // CardKind filter
    // ---------------------------------------------------------------------------

    #[test]
    fn card_kind_pokemon_accepts_pokemon_rejects_trainer() {
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            card_kind: Some(CardKindFilter::Pokemon),
            ..Default::default()
        };

        if let Some(cv) = CardVersion::ALL.iter().find(|c| c.card().is_pokemon()) {
            assert!(filter_card(cv, &cfg, &settings, today(), None, None));
        }
        if let Some(cv) = CardVersion::ALL.iter().find(|c| c.card().is_trainer()) {
            assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
        }
    }

    #[test]
    fn card_kind_trainer_accepts_trainer_rejects_pokemon() {
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            card_kind: Some(CardKindFilter::Trainer),
            ..Default::default()
        };

        if let Some(cv) = CardVersion::ALL.iter().find(|c| c.card().is_trainer()) {
            assert!(filter_card(cv, &cfg, &settings, today(), None, None));
        }
        if let Some(cv) = CardVersion::ALL.iter().find(|c| c.card().is_pokemon()) {
            assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
        }
    }

    // ---------------------------------------------------------------------------
    // Ex filter
    // ---------------------------------------------------------------------------

    #[test]
    fn ex_filter_true_accepts_ex_rejects_non_ex() {
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            ex: Some(true),
            ..Default::default()
        };

        if let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.card().pokemon().is_some_and(|p| p.is_ex()))
        {
            assert!(filter_card(cv, &cfg, &settings, today(), None, None));
        }
        if let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.card().pokemon().is_some_and(|p| !p.is_ex()))
        {
            assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
        }
    }

    #[test]
    fn ex_filter_false_rejects_ex_accepts_non_ex() {
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            ex: Some(false),
            ..Default::default()
        };

        if let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.card().pokemon().is_some_and(|p| p.is_ex()))
        {
            assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
        }
        if let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.card().pokemon().is_some_and(|p| !p.is_ex()))
        {
            assert!(filter_card(cv, &cfg, &settings, today(), None, None));
        }
    }

    // ---------------------------------------------------------------------------
    // Foil filter
    // ---------------------------------------------------------------------------

    #[test]
    fn foil_filter_true_accepts_foil_rejects_non_foil() {
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            foil: Some(true),
            ..Default::default()
        };

        if let Some(cv) = CardVersion::ALL.iter().find(|c| c.is_foil()) {
            assert!(filter_card(cv, &cfg, &settings, today(), None, None));
        }
        if let Some(cv) = CardVersion::ALL.iter().find(|c| !c.is_foil()) {
            assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
        }
    }

    #[test]
    fn foil_filter_false_accepts_non_foil_rejects_foil() {
        let settings = AppSettings::default();
        let cfg = FilterConfig {
            foil: Some(false),
            ..Default::default()
        };

        if let Some(cv) = CardVersion::ALL.iter().find(|c| !c.is_foil()) {
            assert!(filter_card(cv, &cfg, &settings, today(), None, None));
        }
        if let Some(cv) = CardVersion::ALL.iter().find(|c| c.is_foil()) {
            assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
        }
    }

    // ---------------------------------------------------------------------------
    // Source filter
    // ---------------------------------------------------------------------------

    #[test]
    fn source_filter_matching_accepts_non_matching_rejects() {
        let cv = first_cv();
        let src_id = cv.source().id();
        let settings = AppSettings::default();

        let cfg_match = FilterConfig {
            sources: vec![src_id],
            ..Default::default()
        };
        assert!(filter_card(cv, &cfg_match, &settings, today(), None, None));

        // A card from a different source should be rejected.
        let Some(other_cv) = CardVersion::ALL.iter().find(|c| c.source().id() != src_id) else {
            return;
        };
        assert!(!filter_card(
            other_cv,
            &cfg_match,
            &settings,
            today(),
            None,
            None
        ));
    }

    // ---------------------------------------------------------------------------
    // Obtainable filter
    // ---------------------------------------------------------------------------

    #[test]
    fn obtainable_filter_true_rejects_retired_set() {
        // Use a date far in the future so any retirement date is in the past.
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.set().retirement_date().is_some())
        else {
            return;
        };
        let cfg = FilterConfig {
            obtainable: Some(true),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(!filter_card(cv, &cfg, &settings, far_future(), None, None));
    }

    #[test]
    fn obtainable_filter_false_rejects_active_set() {
        // A set with no retirement date is considered active on any date.
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.set().retirement_date().is_none())
        else {
            return;
        };
        let cfg = FilterConfig {
            obtainable: Some(false),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    // ---------------------------------------------------------------------------
    // owned_count threshold
    // ---------------------------------------------------------------------------

    #[test]
    fn owned_count_equal_accepts_exact_rejects_others() {
        let cv = first_cv();
        let cfg = FilterConfig {
            owned_count: Some(CountThreshold::Equal(3)),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, Some(3)));
        assert!(!filter_card(cv, &cfg, &settings, today(), None, Some(2)));
        assert!(!filter_card(cv, &cfg, &settings, today(), None, Some(4)));
    }

    #[test]
    fn owned_count_less_than_accepts_lower_rejects_equal_or_higher() {
        let cv = first_cv();
        let cfg = FilterConfig {
            owned_count: Some(CountThreshold::LessThan(2)),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, Some(0)));
        assert!(filter_card(cv, &cfg, &settings, today(), None, Some(1)));
        assert!(!filter_card(cv, &cfg, &settings, today(), None, Some(2)));
        assert!(!filter_card(cv, &cfg, &settings, today(), None, Some(5)));
    }

    #[test]
    fn owned_count_at_least_accepts_equal_and_higher_rejects_lower() {
        let cv = first_cv();
        let cfg = FilterConfig {
            owned_count: Some(CountThreshold::AtLeast(2)),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, Some(2)));
        assert!(filter_card(cv, &cfg, &settings, today(), None, Some(5)));
        assert!(!filter_card(cv, &cfg, &settings, today(), None, Some(0)));
        assert!(!filter_card(cv, &cfg, &settings, today(), None, Some(1)));
    }

    #[test]
    fn owned_count_threshold_skipped_when_owned_count_arg_is_none() {
        let cv = first_cv();
        // Even with an impossible threshold, passing None skips the check.
        let cfg = FilterConfig {
            owned_count: Some(CountThreshold::Equal(999)),
            ..Default::default()
        };
        let settings = AppSettings::default();
        assert!(filter_card(cv, &cfg, &settings, today(), None, None));
    }

    // ---------------------------------------------------------------------------
    // AppSettings-level filters
    // ---------------------------------------------------------------------------

    #[test]
    fn settings_ignore_premium_mission_rejects_premium_mission_cards() {
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.source().name().as_str() == "Premium Mission")
        else {
            return;
        };
        let mut settings = AppSettings::default();
        settings.set_ignore_premium_mission(true);
        let cfg = FilterConfig::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    #[test]
    fn settings_ignore_gold_shop_rejects_gold_shop_cards() {
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.source().name().as_str() == "Gold Shop")
        else {
            return;
        };
        let mut settings = AppSettings::default();
        settings.set_ignore_gold_shop(true);
        let cfg = FilterConfig::default();
        assert!(!filter_card(cv, &cfg, &settings, today(), None, None));
    }

    #[test]
    fn settings_ignore_unobtainable_sets_rejects_retired_set_cards() {
        let Some(cv) = CardVersion::ALL
            .iter()
            .find(|c| c.set().retirement_date().is_some())
        else {
            return;
        };
        let mut settings = AppSettings::default();
        settings.set_ignore_unobtainable_sets(true);
        let cfg = FilterConfig::default();
        assert!(!filter_card(cv, &cfg, &settings, far_future(), None, None));
    }
}
