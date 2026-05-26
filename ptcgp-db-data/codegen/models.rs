use chrono::NaiveDate;

use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dataset {
    // String tables: sorted alphabetically
    pub rarity_group_names: Vec<String>,
    pub rarity_codes: Vec<String>,
    pub rarity_names: Vec<String>,
    pub series_codes: Vec<String>,
    pub set_codes: Vec<String>,
    pub set_names: Vec<String>,
    pub pack_subtitles: Vec<String>,
    pub card_names: Vec<String>,
    pub card_source_names: Vec<String>,
    pub card_source_descriptions: Vec<String>,
    pub illustrators: Vec<String>,
    pub element_names: Vec<String>,
    pub ability_names: Vec<String>,
    pub ability_effects: Vec<String>,
    pub attack_names: Vec<String>,
    pub attack_effects: Vec<String>,
    pub base_pokemon_names: Vec<String>,
    pub stage_names: Vec<String>,
    pub flavor_text: Vec<String>,
    pub trainer_kind_names: Vec<String>,
    pub trainer_effects: Vec<String>,
    pub pack_variant_names: Vec<String>,

    // Relational data tables: IDs refer to the index within a table
    pub rarity_groups: Vec<RarityGroup>,
    pub rarity_classes: Vec<RarityClass>,
    pub rarities: Vec<Rarity>,
    pub series: Vec<Series>,
    pub sets: Vec<Set>,
    pub packs: Vec<Pack>,
    pub cards: Vec<Card>,
    pub card_versions: Vec<CardVersion>,
    pub card_sources: Vec<CardSource>,
    pub elements: Vec<Element>,
    pub abilities: Vec<Ability>,
    pub attacks: Vec<Attack>,
    pub base_pokemon: Vec<BasePokemon>,
    pub stages: Vec<Stage>,
    pub trainer_kinds: Vec<TrainerKind>,
    pub pack_variants: Vec<PackVariant>,
    pub pack_slots: Vec<PackSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RarityGroup {
    pub id: usize,
    pub name_id: usize,
    pub icon_path: String,
    pub symbol_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RarityClass {
    pub id: usize,
    pub group_id: usize,
    pub count: usize,
    pub icon_path: String,
    pub symbol_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rarity {
    pub id: usize,
    pub group_id: usize,
    pub class_id: usize,
    pub code_id: usize,
    pub name_id: usize,
    pub craft_cost: u32,
    pub dupe_dust: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    pub id: usize,
    pub code_id: usize,
    pub set_ids: Range<usize>,
    pub pack_ids: Range<usize>,
    pub card_version_ids: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub id: usize,
    pub series_id: usize,
    pub code_id: usize,
    pub name_id: usize,
    pub release_date: Option<NaiveDate>,
    pub retirement_date: Option<NaiveDate>,
    pub is_promo: bool,
    pub pack_ids: Range<usize>,
    pub card_version_ids: Range<usize>,
    pub logo_path: String,
    pub icon_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub id: usize,
    pub series_id: usize,
    pub set_id: usize,
    pub subtitle_id: usize,
    pub card_version_ids: Vec<usize>,
    pub variant_ids: Range<usize>,
    pub image_path: String,
    pub logo_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub id: usize,
    pub name_id: usize,
    pub version_ids: Vec<usize>,
    pub kind: CardKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardVersion {
    pub id: usize,
    pub series_id: usize,
    pub set_id: usize,
    pub card_id: usize,
    pub pack_ids: Range<usize>,
    pub number: usize,
    pub rarity_id: usize,
    pub illustrator_id: Option<usize>,
    pub source_id: usize,
    pub is_foil: bool,
    pub is_original: bool,
    pub is_tradable: bool,
    pub duplicate_ids: Vec<usize>,
    pub image_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardSource {
    pub id: usize,
    pub name_id: usize,
    pub description_id: usize,
    pub icon_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardKind {
    Pokemon(PokemonCard),
    Trainer(TrainerCard),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub id: usize,
    pub code: Option<char>,
    pub name_id: usize,
    pub icon_path: String,
    pub symbol_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ability {
    pub id: usize,
    pub name_id: usize,
    pub effect_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attack {
    pub id: usize,
    pub name_id: usize,
    pub effect_id: Option<usize>,
    pub base_damage: u32,
    pub damage_suffix: Option<char>,
    pub cost_element_ids: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasePokemon {
    pub id: usize,
    pub natdex_number: usize,
    pub name_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub id: usize,
    pub name_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PokemonCard {
    pub card_id: usize,
    pub base_id: usize,
    pub element_id: usize,
    pub stage_id: usize,
    pub retreat_cost: u8,
    pub hp: u32,
    pub evolves_from_id: Option<usize>,
    pub flavor_text_id: Option<usize>,
    pub weakness_id: Option<usize>,
    pub ability_id: Option<usize>,
    pub attack_ids: Vec<usize>,
    pub is_ex: bool,
    pub is_mega: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerCard {
    pub card_id: usize,
    pub kind_id: usize,
    pub effect_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerKind {
    pub id: usize,
    pub name_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackVariant {
    pub id: usize,
    pub name_id: usize,
    pub series_id: usize,
    pub set_id: usize,
    pub pack_id: usize,
    pub pull_rate: (u64, u64),
    pub slot_ids: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackSlot {
    pub id: usize,
    pub pack_variant_id: usize,
    pub pull_number: usize,
    pub rarity_pull_rates: Vec<RarityPullRate>,
    pub card_pull_rates: Vec<CardVersionPullRate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RarityPullRate {
    pub rarity_id: usize,
    pub normal: (u64, u64),
    pub foil: (u64, u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardVersionPullRate {
    pub card_version_id: usize,
    pub pull_rate: (u64, u64),
}
