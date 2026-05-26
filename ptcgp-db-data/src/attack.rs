use crate::{
    Element,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

pub struct Attack {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) effect_id: Option<usize>,
    pub(crate) base_damage: u32,
    pub(crate) damage_suffix: Option<char>,
    pub(crate) cost_element_ids: &'static [usize],
}

impl Attack {
    pub const ALL: &[Self] = crate::data::ATTACKS;

    pub const NAMES: &StrTable = crate::data::ATTACK_NAMES;

    pub const EFFECTS: &StrTable = crate::data::ATTACK_EFFECTS;

    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Attack {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    pub const fn from_id(id: usize) -> Option<&'static Attack> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    pub const fn effect(&self) -> Option<StrEntry> {
        if let Some(id) = self.effect_id {
            Some(unsafe { Self::EFFECTS.get_entry_unchecked(id) })
        } else {
            None
        }
    }

    pub const fn base_damage(&self) -> u32 {
        self.base_damage
    }

    pub const fn damage_suffix(&self) -> Option<char> {
        self.damage_suffix
    }

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
