use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

use std::collections::hash_map::{Entry, HashMap};

pub struct RawData {
    pub base_pokemon: Vec<BasePokemon>,
    pub card_sources: Vec<CardSource>,
    pub elements: Vec<Element>,
    pub pack_variant_names: Vec<PackVariantName>,
    pub rarities: Vec<Rarity>,
    pub sets: Vec<Set>,
    pub cards: HashMap<usize, Card>,
    pub card_versions: Vec<CardVersion>,
    pub pack_data: Vec<PackData>,
}

#[derive(Deserialize)]
pub struct BasePokemon {
    pub name: String,
    pub natdex_number: usize,
}

#[derive(Deserialize)]
pub struct CardSource {
    pub code: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct Element {
    #[serde(default)]
    pub symbol: Option<char>,
    pub name: String,
}

#[derive(Deserialize)]
pub struct PackVariantName {
    pub code: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Rarity {
    pub code: String,
    pub name: String,
    pub group: String,
    pub group_symbol_count: usize,
    pub craft_cost: u32,
    pub dupe_dust: u32,
}

#[derive(Deserialize)]
pub struct Set {
    pub code: String,
    pub name: String,
    pub series: String,
    #[serde(default)]
    pub availability: Option<Availability>,
    pub is_promo: bool,
    pub card_count: usize,
    #[serde(default)]
    pub packs: Vec<String>,
}

#[derive(Clone, Copy, Deserialize)]
pub struct Availability {
    pub start: NaiveDate,
    #[serde(default)]
    pub end: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct Card {
    //pub id: usize,
    pub name: String,
    //pub versions: Vec<CardVersionRef>,
    #[serde(flatten)]
    pub kind: CardKind,
}

#[derive(Deserialize)]
pub struct CardVersionRef {
    pub set: String,
    pub number: usize,
}

#[derive(Deserialize)]
#[serde(tag = "card_type")]
pub enum CardKind {
    #[serde(rename = "pokemon")]
    Pokemon {
        #[serde(flatten)]
        pokemon: PokemonCard,
    },
    #[serde(rename = "trainer")]
    Trainer {
        #[serde(flatten)]
        trainer: TrainerCard,
    },
}

#[derive(Deserialize)]
pub struct PokemonCard {
    pub natdex_number: usize,
    pub element: String,
    pub stage: String,
    pub hp: u32,
    pub retreat_cost: u8,
    #[serde(default)]
    pub weakness: Option<String>,
    #[serde(default)]
    pub flavor: Option<String>,
    pub is_ex: bool,
    pub is_mega: bool,
    pub evolves_from: Option<String>,
    #[serde(default)]
    pub ability: Option<Ability>,
    #[serde(default)]
    pub attacks: Vec<Attack>,
}

#[derive(Deserialize)]
pub struct Attack {
    pub name: String,
    #[serde(default)]
    pub cost: Vec<String>,
    pub damage: u32,
    #[serde(default)]
    pub damage_suffix: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
}

#[derive(Deserialize)]
pub struct Ability {
    pub name: String,
    pub effect: String,
}

#[derive(Deserialize)]
pub struct TrainerCard {
    #[serde(rename = "trainer_kind")]
    pub kind: String,
    #[serde(rename = "trainer_effect")]
    pub effect: String,
}

#[derive(Deserialize)]
pub struct CardVersion {
    pub set: String,
    pub number: usize,
    pub card_id: usize,
    pub rarity: String,
    #[serde(default)]
    pub illustrator: Option<String>,
    //pub is_promo: bool,
    pub is_foil: bool,
    pub is_reprint: bool,
    pub is_tradable: bool,
    #[serde(default)]
    pub packs: Vec<String>,
    pub source: String,
    #[serde(default)]
    pub duplicates: Vec<CardVersionRef>,
}

#[derive(Deserialize)]
pub struct PackData {
    pub set: String,
    pub subtitle: String,
    #[serde(default)]
    pub variants: HashMap<String, PackVariant>,
}

#[derive(Deserialize)]
pub struct PackVariant {
    pub rate: Rate,
    pub slot_count: usize,
    #[serde(default)]
    pub rarity_rates_by_slot: Vec<HashMap<String, RarityRates>>,
    #[serde(default)]
    pub card_rates: HashMap<String, Vec<Option<Rate>>>,
}

#[derive(Deserialize)]
pub struct Rate {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Deserialize)]
pub struct RarityRates {
    #[serde(default)]
    pub normal: Rate,
    #[serde(default)]
    pub foil: Rate,
}

impl Default for Rate {
    fn default() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }
}

impl RawData {
    pub fn load() -> Result<Self> {
        let base_pokemon: Vec<BasePokemon> = load_json("base_pokemon.json")?;
        let card_sources: Vec<CardSource> = load_json("card_sources.json")?;
        let elements: Vec<Element> = load_json("elements.json")?;
        let pack_variant_names: Vec<PackVariantName> = load_json("pack_variant_names.json")?;
        let rarities: Vec<Rarity> = load_json("rarities.json")?;
        let sets: Vec<Set> = load_json("sets.json")?;

        let mut card_versions: Vec<CardVersion> =
            Vec::with_capacity(sets.iter().map(|set| set.card_count).sum());
        let mut cards: HashMap<usize, Card> = HashMap::new();
        let mut pack_data: Vec<PackData> =
            Vec::with_capacity(sets.iter().map(|set| set.packs.len()).sum());

        for set in &sets {
            for num in 1..=set.card_count {
                let card_version: CardVersion =
                    load_json(format!("card_versions/{}/{:03}.json", set.code, num))?;
                if let Entry::Vacant(entry) = cards.entry(card_version.card_id) {
                    entry.insert(load_json(format!(
                        "cards/{:05}.json",
                        card_version.card_id
                    ))?);
                }
                card_versions.push(card_version);
            }

            for pack in &set.packs {
                let slug = pack.to_lowercase().replace(" ", "_");
                pack_data.push(load_json(format!("pull_rates/{}/{}.json", set.code, slug))?);
            }
        }

        Ok(RawData {
            base_pokemon,
            card_sources,
            elements,
            pack_variant_names,
            rarities,
            sets,
            cards,
            card_versions,
            pack_data,
        })
    }
}

fn load_json<T>(path: impl AsRef<std::path::Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let base: &std::path::Path = "ptcgp-data/data".as_ref();
    let path = base.join(path.as_ref());
    println!("cargo:rerun-if-changed={}", path.display());
    let json = match std::fs::read(&path) {
        Ok(json) => json,
        Err(err) => {
            let msg = format!("failed to read JSON data from {}", path.display());
            return Err::<T, _>(err).context(msg);
        }
    };
    match serde_json::from_slice::<T>(&json) {
        Ok(data) => Ok(data),
        err => {
            let msg = format!("failed to decode JSON data from {}", path.display());
            err.context(msg)
        }
    }
}
