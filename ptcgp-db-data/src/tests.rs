//! Data integrity tests for the generated ptcgp-db-data constants.
//!
//! These tests verify structural invariants that the code-generator is expected to maintain.
//! Each test catches a distinct class of codegen bug; none of them test trivial getters.

use crate::{
    Ability, Attack, BasePokemon, Card, CardSource, CardVersion, Element, Pack, PackSlot,
    PackVariant, Prob, Rarity, RarityClass, RarityGroup, Series, Set, Stage, TrainerKind,
};

// ── ID self-consistency ──────────────────────────────────────────────────────

/// Every entry's `id()` must equal its index in the corresponding `ALL` slice.
/// A mismatch means the codegen emitted wrong index offsets somewhere.
#[test]
fn id_self_consistency() {
    for (i, x) in Series::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Series id mismatch at index {i}");
    }
    for (i, x) in Set::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Set id mismatch at index {i}");
    }
    for (i, x) in Pack::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Pack id mismatch at index {i}");
    }
    for (i, x) in PackVariant::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "PackVariant id mismatch at index {i}");
    }
    for (i, x) in PackSlot::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "PackSlot id mismatch at index {i}");
    }
    for (i, x) in Card::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Card id mismatch at index {i}");
    }
    for (i, x) in CardVersion::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "CardVersion id mismatch at index {i}");
    }
    for (i, x) in Rarity::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Rarity id mismatch at index {i}");
    }
    for (i, x) in RarityClass::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "RarityClass id mismatch at index {i}");
    }
    for (i, x) in RarityGroup::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "RarityGroup id mismatch at index {i}");
    }
    for (i, x) in Element::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Element id mismatch at index {i}");
    }
    for (i, x) in Stage::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Stage id mismatch at index {i}");
    }
    for (i, x) in TrainerKind::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "TrainerKind id mismatch at index {i}");
    }
    for (i, x) in CardSource::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "CardSource id mismatch at index {i}");
    }
    for (i, x) in BasePokemon::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "BasePokemon id mismatch at index {i}");
    }
    for (i, x) in Attack::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Attack id mismatch at index {i}");
    }
    for (i, x) in Ability::ALL.iter().enumerate() {
        assert_eq!(x.id(), i, "Ability id mismatch at index {i}");
    }
}

// ── Pull-rate arithmetic invariants ─────────────────────────────────────────

/// For every pack, the sum of all variant pull rates must equal exactly 1.
/// This is the foundation of the probability calculations throughout the app.
#[test]
fn pack_variant_rates_sum_to_one() {
    for pack in Pack::ALL {
        if pack.variants().is_empty() {
            continue;
        }
        let sum = pack
            .variants()
            .iter()
            .fold(Prob::ZERO, |acc, v| acc + v.pull_rate());
        assert_eq!(
            sum,
            Prob::ONE,
            "Pack {} ({}) variant rates sum to {:#} instead of 1",
            pack.id(),
            pack.title(),
            sum,
        );
    }
}

/// Each slot's `pull_number` must equal its position in the variant's slot list.
/// A mismatch means slot ordering was scrambled during codegen.
#[test]
fn slot_pull_numbers_are_sequential() {
    for variant in PackVariant::ALL {
        for (i, slot) in variant.slots().iter().enumerate() {
            assert_eq!(
                slot.pull_number(),
                i,
                "Slot pull_number {} ≠ position {i} in variant {}",
                slot.pull_number(),
                variant.id(),
            );
        }
    }
}

// ── Cross-reference consistency ──────────────────────────────────────────────

/// If a pack lists a card version in its pool, that card version must list the pack back.
#[test]
fn pack_to_card_version_cross_reference() {
    for pack in Pack::ALL {
        for cv in pack.card_versions() {
            assert!(
                cv.packs().iter().any(|p| p.id() == pack.id()),
                "Pack {} ({}) contains CardVersion {} but cv.packs() doesn't include it",
                pack.id(),
                pack.title(),
                cv.id(),
            );
        }
    }
}

/// If a card version lists a pack, that pack must list the card version back.
#[test]
fn card_version_to_pack_cross_reference() {
    for cv in CardVersion::ALL {
        for pack in cv.packs() {
            assert!(
                pack.card_versions().iter().any(|c| c.id() == cv.id()),
                "CardVersion {} lists Pack {} ({}) but pack.card_versions() doesn't include it",
                cv.id(),
                pack.id(),
                pack.title(),
            );
        }
    }
}

/// Every card version must appear in its abstract card's `versions()` list.
#[test]
fn card_version_in_abstract_card_versions() {
    for cv in CardVersion::ALL {
        assert!(
            cv.card().versions().iter().any(|v| v.id() == cv.id()),
            "CardVersion {} not found in card {}'s versions()",
            cv.id(),
            cv.card().id(),
        );
    }
}

/// Every set must appear in its series' `sets()` list, and every set in a
/// series' `sets()` list must point back to that series.
#[test]
fn series_set_cross_reference() {
    for series in Series::ALL {
        for set in series.sets() {
            assert_eq!(
                set.series().id(),
                series.id(),
                "Set {:?} is in series {:?}'s sets() but set.series() points elsewhere",
                set.code(),
                series.code(),
            );
        }
    }
    for set in Set::ALL {
        assert!(
            set.series().sets().iter().any(|s| s.id() == set.id()),
            "Set {:?} not found in its series' sets()",
            set.code(),
        );
    }
}

// ── Duplicate-group invariants ────────────────────────────────────────────────

/// The duplicate relationship must be symmetric: if B is in A's duplicates,
/// then A must be in B's duplicates.
#[test]
fn duplicate_symmetry() {
    for cv in CardVersion::ALL {
        for dup in cv.duplicates() {
            assert!(
                dup.duplicates().iter().any(|d| d.id() == cv.id()),
                "CardVersion {} is in {}'s duplicates but not vice versa",
                cv.id(),
                dup.id(),
            );
        }
    }
}

/// For every duplicate group (card + its duplicates), exactly one member must
/// have `is_original() == true`. Having zero or more than one breaks the
/// "prefer the original version" logic used in trade recommendations.
#[test]
fn is_original_unique_per_duplicate_group() {
    for cv in CardVersion::ALL {
        if cv.duplicates().is_empty() {
            continue;
        }
        let originals = std::iter::once(cv)
            .chain(cv.duplicates().iter())
            .filter(|v| v.is_original())
            .count();
        assert_eq!(
            originals,
            1,
            "Duplicate group containing CardVersion {} has {originals} original(s) (expected 1)",
            cv.id(),
        );
    }
}

// ── Card-source invariants ────────────────────────────────────────────────────

/// Cards whose source is not "Pack" must have an empty pack list — they have
/// no pull rate data by definition.
#[test]
fn non_pack_source_has_no_packs() {
    for cv in CardVersion::ALL {
        if cv.source().name().as_str() != "Pack" {
            assert!(
                cv.packs().is_empty(),
                "CardVersion {} has source {:?} but packs() is non-empty",
                cv.id(),
                cv.source().name().as_str(),
            );
        }
    }
}

// ── Set-availability invariants ───────────────────────────────────────────────

/// Promo sets must have no release or retirement date; availability is tracked
/// per-card, not per-set, for promos.
#[test]
fn promo_sets_have_no_dates() {
    for set in Set::ALL {
        if set.is_promo() {
            assert_eq!(
                set.release_date(),
                None,
                "Promo set {:?} unexpectedly has a release_date",
                set.code(),
            );
            assert_eq!(
                set.retirement_date(),
                None,
                "Promo set {:?} unexpectedly has a retirement_date",
                set.code(),
            );
        }
    }
}

/// Non-promo sets must have a release date — they became available on a
/// specific calendar date.
#[test]
fn non_promo_sets_have_release_dates() {
    for set in Set::ALL {
        if !set.is_promo() {
            assert!(
                set.release_date().is_some(),
                "Non-promo set {:?} is missing a release_date",
                set.code(),
            );
        }
    }
}

// ── Element invariant ────────────────────────────────────────────────────────

/// Dragon is the only element with `code() == None`. All other elements,
/// including Colorless, must have a code because they represent real energies
/// that appear in effect-text placeholders.
#[test]
fn dragon_is_only_element_without_code() {
    for elem in Element::ALL {
        if elem.name().as_str() == "Dragon" {
            assert_eq!(
                elem.code(),
                None,
                "Dragon element should have no energy code but has {:?}",
                elem.code(),
            );
        } else {
            assert!(
                elem.code().is_some(),
                "Element {:?} is missing an energy code",
                elem.name().as_str(),
            );
        }
    }
}

