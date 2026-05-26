use crate::{CardVersion, PackVariant, Prob, Rarity};

pub struct PackSlot {
    pub(crate) id: usize,
    pub(crate) variant_id: usize,
    pub(crate) pull_number: usize,
    pub(crate) rarities: &'static [RarityPullRate],
    pub(crate) card_versions: &'static [CardVersionPullRate],
}

pub struct RarityPullRate {
    pub(crate) rarity_id: usize,
    pub(crate) normal: Prob,
    pub(crate) foil: Prob,
}

pub struct CardVersionPullRate {
    pub(crate) card_version_id: usize,
    pub(crate) pull_rate: Prob,
}

impl PackSlot {
    pub const ALL: &[Self] = crate::data::PACK_SLOTS;

    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn variant(&self) -> &'static PackVariant {
        unsafe { PackVariant::from_id_unchecked(self.variant_id) }
    }

    pub const fn pull_number(&self) -> usize {
        self.pull_number
    }

    pub const fn rarities(&self) -> &'static [RarityPullRate] {
        self.rarities
    }

    pub const fn card_versions(&self) -> &'static [CardVersionPullRate] {
        self.card_versions
    }
}

impl std::fmt::Debug for PackSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackSlot")
            .field("id", &self.id)
            .field("set", &self.variant().pack().set().code())
            .field("pack", &self.variant().pack().subtitle())
            .field("variant", &self.variant().name())
            .field("pull_number", &self.pull_number)
            .field("rarity_pull_rates", &self.rarities)
            .field("card_version_pull_rates", &self.card_versions)
            .finish()
    }
}

impl PartialEq for PackSlot {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PackSlot {}

impl PartialOrd for PackSlot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for PackSlot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for PackSlot {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for PackSlot {
    const INDEXED: &[Self] = Self::ALL;
}

impl RarityPullRate {
    pub const fn rarity(&self) -> &'static Rarity {
        unsafe { Rarity::from_id_unchecked(self.rarity_id) }
    }

    pub const fn normal_pull_rate(&self) -> Prob {
        self.normal
    }

    pub const fn foil_pull_rate(&self) -> Prob {
        self.foil
    }
}

impl std::fmt::Debug for RarityPullRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RarityPullRate")
            .field("rarity", &self.rarity().code())
            .field("normal_pull_rate", &self.normal)
            .field("foil_pull_rate", &self.foil)
            .finish()
    }
}

impl CardVersionPullRate {
    pub const fn card_version(&self) -> &'static CardVersion {
        unsafe { CardVersion::from_id_unchecked(self.card_version_id) }
    }

    pub const fn pull_rate(&self) -> Prob {
        self.pull_rate
    }
}

impl std::fmt::Debug for CardVersionPullRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardVersionPullRate")
            .field(
                "card_version",
                &format_args!(
                    "{}-{:03}",
                    self.card_version().set().code(),
                    self.card_version().number()
                ),
            )
            .field("pull_rate", &self.pull_rate)
            .finish()
    }
}
