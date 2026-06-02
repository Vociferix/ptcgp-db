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
