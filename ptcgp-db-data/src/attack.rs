//! Card attacks.

use crate::{
    Element,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

/// A card attack, usable by one or more Pokémon cards.
pub struct Attack {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) effect_id: Option<usize>,
    pub(crate) base_damage: u32,
    pub(crate) damage_suffix: Option<char>,
    pub(crate) cost_element_ids: &'static [usize],
}

impl Attack {
    /// All attacks, sorted by ID.
    pub const ALL: &[Self] = crate::data::ATTACKS;

    /// Attack name strings.
    pub const NAMES: &StrTable = crate::data::ATTACK_NAMES;

    /// Attack effect text strings.
    pub const EFFECTS: &StrTable = crate::data::ATTACK_EFFECTS;

    /// Returns the attack with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Attack {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the attack with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Attack> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Attack::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Attack name.
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Effect text, if the attack has one. May contain element placeholders (e.g., `[R]` for
    /// Fire) intended to be replaced with [`Element`] symbol images in the UI.
    pub const fn effect(&self) -> Option<StrEntry> {
        if let Some(id) = self.effect_id {
            Some(unsafe { Self::EFFECTS.get_entry_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric base damage value, not accounting for the suffix character.
    pub const fn base_damage(&self) -> u32 {
        self.base_damage
    }

    /// Optional suffix character appended to the damage display. Known values:
    /// - `'+'` — attack can do additional damage (condition described in the effect text)
    /// - `'-'` — attack can do reduced damage (condition described in the effect text)
    /// - `'×'` — base damage applies multiple times (typically coin-flip based, described in
    ///   the effect text)
    ///
    /// `None` means the attack deals exactly the base damage with no variation.
    pub const fn damage_suffix(&self) -> Option<char> {
        self.damage_suffix
    }

    /// Formats the full damage string as `"{base_damage}{suffix}"` (e.g., `"120+"`, `"50"`).
    pub const fn damage(&self) -> impl std::fmt::Display {
        struct FmtDamage {
            base: u32,
            suffix: Option<char>,
        }

        impl std::fmt::Display for FmtDamage {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.base, f)?;
                if let Some(suffix) = self.suffix {
                    std::fmt::Display::fmt(&suffix, f)
                } else {
                    Ok(())
                }
            }
        }

        FmtDamage {
            base: self.base_damage,
            suffix: self.damage_suffix,
        }
    }

    /// Energy cost as an ordered list of [`Element`]s. Elements may repeat for multi-energy
    /// costs (e.g., three Fire entries for a 3-Fire cost). An empty list means zero cost;
    /// display [`Element::NO_COST`] in that case.
    pub const fn cost(&self) -> &'static IdSlice<Element> {
        unsafe { IdSlice::new_unchecked(self.cost_element_ids) }
    }
}

impl std::fmt::Debug for Attack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attack")
            .field("id", &self.id)
            .field("name", &self.name())
            .field("effect", &self.effect())
            .field("base_damage", &self.base_damage)
            .field("damage_suffix", &self.damage_suffix)
            .finish()
    }
}

impl PartialEq for Attack {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Attack {}

impl PartialOrd for Attack {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Attack {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Attack {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Attack {
    const INDEXED: &'static [Self] = Self::ALL;
}
