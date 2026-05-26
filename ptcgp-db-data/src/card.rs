//! Abstract card identity and kind-specific data.

use crate::{
    Ability, Attack, BasePokemon, CardVersion, Element, Stage, TrainerKind,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

/// The game-mechanical identity of a card: name, stats, attacks, and effects.
///
/// One abstract card may be printed as many different physical [`CardVersion`]s across
/// different sets. Owned counts are always tracked per `CardVersion`, not per `Card`.
pub struct Card {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) version_ids: &'static [usize],
    pub(crate) kind: CardKind,
}

/// Whether a card is a Pokémon or Trainer, with kind-specific data attached.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardKind {
    Pokemon(PokemonCard),
    Trainer(TrainerCard),
}

/// Pokémon-specific data for an abstract card.
pub struct PokemonCard {
    pub(crate) card_id: usize,
    pub(crate) base_id: usize,
    pub(crate) element_id: usize,
    pub(crate) stage_id: usize,
    pub(crate) retreat_cost: u8,
    pub(crate) hp: u32,
    pub(crate) evolves_from_id: Option<usize>,
    pub(crate) flavor_text_id: Option<usize>,
    pub(crate) weakness_id: Option<usize>,
    pub(crate) ability_id: Option<usize>,
    pub(crate) attack_ids: &'static [usize],
    pub(crate) is_ex: bool,
    pub(crate) is_mega: bool,
}

/// Trainer-specific data for an abstract card.
pub struct TrainerCard {
    pub(crate) card_id: usize,
    pub(crate) kind_id: usize,
    pub(crate) effect_id: usize,
}

impl Card {
    /// All abstract cards, sorted by ID.
    pub const ALL: &[Self] = crate::data::CARDS;

    /// Card name strings, shared by both Pokémon and Trainer cards.
    pub const NAMES: &StrTable = crate::data::CARD_NAMES;

    /// Returns the card with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the card with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Card::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Display name of the card.
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// All [`CardVersion`]s that share this abstract card's game mechanics (same name, stats,
    /// attacks, and effects but possibly different sets, rarities, or illustrators).
    pub const fn versions(&self) -> &'static IdSlice<CardVersion> {
        unsafe { IdSlice::new_unchecked(self.version_ids) }
    }

    /// Pokémon or Trainer discriminant with kind-specific data attached.
    pub const fn kind(&self) -> &CardKind {
        &self.kind
    }

    /// True if this is a Pokémon card.
    pub const fn is_pokemon(&self) -> bool {
        matches!(&self.kind, CardKind::Pokemon(_))
    }

    /// True if this is a Trainer card.
    pub const fn is_trainer(&self) -> bool {
        matches!(&self.kind, CardKind::Trainer(_))
    }

    /// Returns the Pokémon-specific data, or `None` if this is a Trainer card.
    pub const fn pokemon(&self) -> Option<&PokemonCard> {
        match &self.kind {
            CardKind::Pokemon(pkmn) => Some(pkmn),
            _ => None,
        }
    }

    /// Returns the Trainer-specific data, or `None` if this is a Pokémon card.
    pub const fn trainer(&self) -> Option<&TrainerCard> {
        match &self.kind {
            CardKind::Trainer(tr) => Some(tr),
            _ => None,
        }
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Card")
            .field("id", &self.id)
            .field("name", &self.name())
            .field("kind", &self.kind)
            .finish()
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Card {}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Card {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl std::hash::Hash for CardKind {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        match self {
            Self::Pokemon(pkmn) => pkmn.card_id.hash(state),
            Self::Trainer(tr) => tr.card_id.hash(state),
        }
    }
}

impl PokemonCard {
    /// Flavor text strings shared across all Pokémon cards.
    pub const FLAVOR_TEXT: &StrTable = crate::data::FLAVOR_TEXT;

    /// Returns the Pokémon card with the given card ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Card::ALL.len()` and the card at that index must be a Pokémon
    /// card (not a Trainer).
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        let card = unsafe { Card::from_id_unchecked(id) };
        let CardKind::Pokemon(pkmn) = &card.kind else {
            unsafe {
                std::hint::unreachable_unchecked();
            }
        };
        pkmn
    }

    /// Returns the Pokémon card for the given card ID, or `None` if out of range or not a
    /// Pokémon card.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if let Some(card) = Card::from_id(id)
            && let Some(pkmn) = card.pokemon()
        {
            Some(pkmn)
        } else {
            None
        }
    }

    /// Numeric index into [`Card::ALL`], shared with the parent [`Card`].
    pub const fn id(&self) -> usize {
        self.card_id
    }

    /// The parent abstract [`Card`].
    pub const fn card(&self) -> &'static Card {
        unsafe { Card::from_id_unchecked(self.card_id) }
    }

    /// National Pokédex entry for this Pokémon species.
    pub const fn base_pokemon(&self) -> &'static BasePokemon {
        unsafe { BasePokemon::from_id_unchecked(self.base_id) }
    }

    /// Energy type of this Pokémon.
    pub const fn element(&self) -> &'static Element {
        unsafe { Element::from_id_unchecked(self.element_id) }
    }

    /// Evolution stage: Basic, Stage 1, or Stage 2.
    pub const fn stage(&self) -> &'static Stage {
        unsafe { Stage::from_id_unchecked(self.stage_id) }
    }

    /// Number of energies required to retreat.
    pub const fn retreat_cost(&self) -> u8 {
        self.retreat_cost
    }

    /// Hit points.
    pub const fn hp(&self) -> u32 {
        self.hp
    }

    /// Name of the card this card evolves from, if any.
    ///
    /// The returned string entry corresponds to [`Card::NAMES`], so this
    /// card can evolve from any other card whose `Card::name()` returns
    /// this entry. While this is usually the name of a Pokémon card, there
    /// are cases where this refers to a Trainer card (such as a Fossil).
    pub const fn evolves_from(&self) -> Option<StrEntry> {
        if let Some(id) = self.evolves_from_id {
            Some(unsafe { Card::NAMES.get_entry_unchecked(id) })
        } else {
            None
        }
    }

    /// Flavor text, if present.
    pub const fn flavor_text(&self) -> Option<StrEntry> {
        if let Some(id) = self.flavor_text_id {
            Some(unsafe { Self::FLAVOR_TEXT.get_entry_unchecked(id) })
        } else {
            None
        }
    }

    /// Weakness element, if any.
    pub const fn weakness(&self) -> Option<&'static Element> {
        if let Some(id) = self.weakness_id {
            Some(unsafe { Element::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Pokémon ability, if present.
    pub const fn ability(&self) -> Option<&'static Ability> {
        if let Some(id) = self.ability_id {
            Some(unsafe { Ability::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Attacks this card can use. Most Pokémon cards have one or two attacks.
    pub const fn attacks(&self) -> &'static IdSlice<Attack> {
        unsafe { IdSlice::new_unchecked(self.attack_ids) }
    }

    /// True for Pokémon ex cards.
    pub const fn is_ex(&self) -> bool {
        self.is_ex
    }

    /// True for Mega Pokémon ex cards.
    pub const fn is_mega(&self) -> bool {
        self.is_mega
    }
}

impl std::fmt::Debug for PokemonCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PokemonCard")
            .field("id", &self.card_id)
            .field("base_pokemon", self.base_pokemon())
            .field("element", self.element())
            .field("stage", self.stage())
            .field("retreat_cost", &self.retreat_cost)
            .field("hp", &self.hp)
            .field("evolves_from", &self.evolves_from())
            .field("flavor_text", &self.flavor_text())
            .field("weakness", &self.weakness())
            .field("ability", &self.ability())
            .field("attacks", &self.attacks())
            .field("is_ex", &self.is_ex)
            .field("is_mega", &self.is_mega)
            .finish()
    }
}

impl PartialEq for PokemonCard {
    fn eq(&self, other: &Self) -> bool {
        self.card_id == other.card_id
    }
}

impl Eq for PokemonCard {}

impl PartialOrd for PokemonCard {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for PokemonCard {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.card_id, &other.card_id)
    }
}

impl std::hash::Hash for PokemonCard {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.card_id.hash(state);
    }
}

impl TrainerCard {
    /// Effect text strings for all Trainer cards.
    pub const EFFECTS: &StrTable = crate::data::TRAINER_EFFECTS;

    /// Returns the Trainer card with the given card ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Card::ALL.len()` and the card at that index must be a Trainer
    /// card (not a Pokémon).
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        let card = unsafe { Card::from_id_unchecked(id) };
        let CardKind::Trainer(tr) = &card.kind else {
            unsafe {
                std::hint::unreachable_unchecked();
            }
        };
        tr
    }

    /// Returns the Trainer card for the given card ID, or `None` if out of range or not a
    /// Trainer card.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if let Some(card) = Card::from_id(id)
            && let Some(tr) = card.trainer()
        {
            Some(tr)
        } else {
            None
        }
    }

    /// Numeric index into [`Card::ALL`], shared with the parent [`Card`].
    pub const fn id(&self) -> usize {
        self.card_id
    }

    /// The parent abstract [`Card`].
    pub const fn card(&self) -> &'static Card {
        unsafe { Card::from_id_unchecked(self.card_id) }
    }

    /// Category of this Trainer card (Item, Supporter, Stadium, or Tool).
    pub const fn kind(&self) -> &'static TrainerKind {
        unsafe { TrainerKind::from_id_unchecked(self.kind_id) }
    }

    /// Effect text. May contain element placeholders (e.g., `[R]` for Fire) intended to be
    /// replaced with [`Element`] symbol images in the UI.
    ///
    /// [`Element`]: crate::Element
    pub const fn effect(&self) -> StrEntry {
        unsafe { Self::EFFECTS.get_entry_unchecked(self.effect_id) }
    }
}

impl std::fmt::Debug for TrainerCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainerCard")
            .field("id", &self.card_id)
            .field("kind", self.kind())
            .field("effect", &self.effect())
            .finish()
    }
}

impl PartialEq for TrainerCard {
    fn eq(&self, other: &Self) -> bool {
        self.card_id == other.card_id
    }
}

impl Eq for TrainerCard {}

impl PartialOrd for TrainerCard {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for TrainerCard {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.card_id, &other.card_id)
    }
}

impl std::hash::Hash for TrainerCard {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.card_id.hash(state);
    }
}

impl crate::id_slice::Indexed for Card {
    const INDEXED: &'static [Self] = Self::ALL;
}
