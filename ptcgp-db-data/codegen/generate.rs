use crate::models::{CardKind, Dataset, PokemonCard, TrainerCard};

use chrono::{Datelike, NaiveDate};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate(data: Dataset) -> TokenStream {
    let strings = gen_strings(&data);
    let rarity_groups = gen_rarity_groups(&data);
    let rarity_classes = gen_rarity_classes(&data);
    let rarities = gen_rarities(&data);
    let series = gen_series(&data);
    let sets = gen_sets(&data);
    let packs = gen_packs(&data);
    let cards = gen_cards(&data);
    let card_versions = gen_card_versions(&data);
    let card_sources = gen_card_sources(&data);
    let elements = gen_elements(&data);
    let abilities = gen_abilities(&data);
    let attacks = gen_attacks(&data);
    let base_pokemon = gen_base_pokemon(&data);
    let stages = gen_stages(&data);
    let trainer_kinds = gen_trainer_kinds(&data);
    let pack_variants = gen_pack_variants(&data);
    let pack_slots = gen_pack_slots(&data);

    quote! {
        #strings
        #rarity_groups
        #rarity_classes
        #rarities
        #series
        #sets
        #packs
        #cards
        #card_versions
        #card_sources
        #elements
        #abilities
        #attacks
        #base_pokemon
        #stages
        #trainer_kinds
        #pack_variants
        #pack_slots
    }
}

fn gen_strings(data: &Dataset) -> TokenStream {
    let rarity_group_names = gen_string_table("RARITY_GROUP_NAMES", &data.rarity_group_names);
    let rarity_codes = gen_string_table("RARITY_CODES", &data.rarity_codes);
    let rarity_names = gen_string_table("RARITY_NAMES", &data.rarity_names);
    let series_codes = gen_string_table("SERIES_CODES", &data.series_codes);
    let set_codes = gen_string_table("SET_CODES", &data.set_codes);
    let set_names = gen_string_table("SET_NAMES", &data.set_names);
    let pack_subtitles = gen_string_table("PACK_SUBTITLES", &data.pack_subtitles);
    let card_names = gen_string_table("CARD_NAMES", &data.card_names);
    let card_source_names = gen_string_table("CARD_SOURCE_NAMES", &data.card_source_names);
    let card_source_descriptions =
        gen_string_table("CARD_SOURCE_DESCRIPTIONS", &data.card_source_descriptions);
    let illustrators = gen_string_table("ILLUSTRATORS", &data.illustrators);
    let element_names = gen_string_table("ELEMENT_NAMES", &data.element_names);
    let ability_names = gen_string_table("ABILITY_NAMES", &data.ability_names);
    let ability_effects = gen_string_table("ABILITY_EFFECTS", &data.ability_effects);
    let attack_names = gen_string_table("ATTACK_NAMES", &data.attack_names);
    let attack_effects = gen_string_table("ATTACK_EFFECTS", &data.attack_effects);
    let base_pokemon_names = gen_string_table("BASE_POKEMON_NAMES", &data.base_pokemon_names);
    let stage_names = gen_string_table("STAGE_NAMES", &data.stage_names);
    let flavor_text = gen_string_table("FLAVOR_TEXT", &data.flavor_text);
    let trainer_kind_names = gen_string_table("TRAINER_KIND_NAMES", &data.trainer_kind_names);
    let trainer_effects = gen_string_table("TRAINER_EFFECTS", &data.trainer_effects);
    let pack_variant_names = gen_string_table("PACK_VARIANT_NAMES", &data.pack_variant_names);

    quote! {
        #rarity_group_names
        #rarity_codes
        #rarity_names
        #series_codes
        #set_codes
        #set_names
        #pack_subtitles
        #card_names
        #card_source_names
        #card_source_descriptions
        #illustrators
        #element_names
        #ability_names
        #ability_effects
        #attack_names
        #attack_effects
        #base_pokemon_names
        #stage_names
        #flavor_text
        #trainer_kind_names
        #trainer_effects
        #pack_variant_names
    }
}

fn gen_string_table(name: &str, table: &[String]) -> TokenStream {
    let var_name = format_ident!("{name}");
    let lower_strings = table.iter().map(|s| s.to_lowercase());

    quote! {
        pub static #var_name: &str_table::StrTable = unsafe {
            &str_table::StrTable::new_unchecked(
                &[#(#table,)*],
                &[#(#lower_strings,)*],
            )
        };
    }
}

fn gen_rarity_groups(data: &Dataset) -> TokenStream {
    let groups = data.rarity_groups.iter().map(|group| {
        let id = group.id;
        let name_id = group.name_id;
        let icon_path = &group.icon_path;
        let symbol_path = &group.symbol_path;

        quote! {
            RarityGroup {
                id: #id,
                name_id: #name_id,
                #[cfg(feature = "images")]
                icon: asset!(#icon_path),
                #[cfg(feature = "images")]
                symbol: asset!(#symbol_path),
            }
        }
    });

    quote! {
        pub const RARITY_GROUPS: &[RarityGroup] = &[#(#groups,)*];
    }
}

fn gen_rarity_classes(data: &Dataset) -> TokenStream {
    let classes = data.rarity_classes.iter().map(|class| {
        let id = class.id;
        let group_id = class.group_id;
        let count = class.count;
        let icon_path = &class.icon_path;
        let symbol_path = &class.symbol_path;

        quote! {
            RarityClass {
                id: #id,
                group_id: #group_id,
                count: #count,
                #[cfg(feature = "images")]
                icon: asset!(#icon_path),
                #[cfg(feature = "images")]
                symbol: asset!(#symbol_path),
            }
        }
    });

    quote! {
        pub const RARITY_CLASSES: &[RarityClass] = &[#(#classes,)*];
    }
}

fn gen_rarities(data: &Dataset) -> TokenStream {
    let rarities = data.rarities.iter().map(|rarity| {
        let id = rarity.id;
        let code_id = rarity.code_id;
        let name_id = rarity.name_id;
        let group_id = rarity.group_id;
        let class_id = rarity.class_id;
        let craft_cost = rarity.craft_cost;
        let dupe_dust = rarity.dupe_dust;

        quote! {
            Rarity {
                id: #id,
                code_id: #code_id,
                name_id: #name_id,
                class_id: #class_id,
                group_id: #group_id,
                craft_cost: #craft_cost,
                dupe_dust: #dupe_dust,
            }
        }
    });

    quote! {
        pub const RARITIES: &[Rarity] = &[#(#rarities,)*];
    }
}

fn gen_series(data: &Dataset) -> TokenStream {
    let series = data.series.iter().map(|series| {
        let id = series.id;
        let code_id = series.code_id;
        let sets_start = series.set_ids.start;
        let sets_end = series.set_ids.end;
        let packs_start = series.pack_ids.start;
        let packs_end = series.pack_ids.end;
        let cards_start = series.card_version_ids.start;
        let cards_end = series.card_version_ids.end;

        quote! {
            Series {
                id: #id,
                code_id: #code_id,
                set_ids: #sets_start..#sets_end,
                pack_ids: #packs_start..#packs_end,
                card_version_ids: #cards_start..#cards_end,
            }
        }
    });

    quote! {
        pub const SERIES: &[Series] = &[#(#series,)*];
    }
}

fn gen_sets(data: &Dataset) -> TokenStream {
    let sets = data.sets.iter().map(|set| {
        let id = set.id;
        let series_id = set.series_id;
        let code_id = set.code_id;
        let name_id = set.name_id;
        let release_date = gen_date_opt(set.release_date);
        let retirement_date = gen_date_opt(set.retirement_date);
        let is_promo = set.is_promo;
        let packs_start = set.pack_ids.start;
        let packs_end = set.pack_ids.end;
        let cards_start = set.card_version_ids.start;
        let cards_end = set.card_version_ids.end;
        let logo_path = &set.logo_path;
        let icon_path = &set.icon_path;

        quote! {
            Set {
                id: #id,
                series_id: #series_id,
                code_id: #code_id,
                name_id: #name_id,
                release_date: #release_date,
                retirement_date: #retirement_date,
                is_promo: #is_promo,
                pack_ids: #packs_start..#packs_end,
                card_version_ids: #cards_start..#cards_end,
                #[cfg(feature = "images")]
                logo: asset!(#logo_path),
                #[cfg(feature = "images")]
                icon: asset!(#icon_path),
            }
        }
    });

    quote! {
        pub const SETS: &[Set] = &[#(#sets,)*];
    }
}

fn gen_packs(data: &Dataset) -> TokenStream {
    let packs = data.packs.iter().map(|pack| {
        let id = pack.id;
        let series_id = pack.series_id;
        let set_id = pack.set_id;
        let subtitle_id = pack.subtitle_id;
        let card_ids = pack.card_version_ids.as_slice();
        let variants_start = pack.variant_ids.start;
        let variants_end = pack.variant_ids.end;
        let image_path = &pack.image_path;
        let logo_path = &pack.logo_path;

        quote! {
            Pack {
                id: #id,
                series_id: #series_id,
                set_id: #set_id,
                subtitle_id: #subtitle_id,
                card_version_ids: &[#(#card_ids,)*],
                variant_ids: #variants_start..#variants_end,
                #[cfg(feature = "images")]
                image: asset!(#image_path),
                #[cfg(feature = "images")]
                logo: asset!(#logo_path),
            }
        }
    });

    quote! {
        pub const PACKS: &[Pack] = &[#(#packs,)*];
    }
}

fn gen_cards(data: &Dataset) -> TokenStream {
    let cards = data.cards.iter().map(|card| {
        let id = card.id;
        let name_id = card.name_id;
        let version_ids = card.version_ids.as_slice();
        let kind = match &card.kind {
            CardKind::Pokemon(pkmn) => gen_pokemon_card(pkmn),
            CardKind::Trainer(tr) => gen_trainer_card(tr),
        };

        quote! {
            Card {
                id: #id,
                name_id: #name_id,
                version_ids: &[#(#version_ids,)*],
                kind: #kind,
            }
        }
    });

    quote! {
        pub const CARDS: &[Card] = &[#(#cards,)*];
    }
}

fn gen_pokemon_card(pkmn: &PokemonCard) -> TokenStream {
    let card_id = pkmn.card_id;
    let base_id = pkmn.base_id;
    let element_id = pkmn.element_id;
    let stage_id = pkmn.stage_id;
    let retreat_cost = pkmn.retreat_cost;
    let hp = pkmn.hp;
    let evolves_from_id = if let Some(id) = pkmn.evolves_from_id {
        quote! { Some(#id) }
    } else {
        quote! { None }
    };
    let flavor_text_id = if let Some(id) = pkmn.flavor_text_id {
        quote! { Some(#id) }
    } else {
        quote! { None }
    };
    let weakness = if let Some(id) = pkmn.weakness_id {
        quote! { Some(#id) }
    } else {
        quote! { None }
    };
    let ability = if let Some(id) = pkmn.ability_id {
        quote! { Some(#id) }
    } else {
        quote! { None }
    };
    let attack_ids = pkmn.attack_ids.as_slice();
    let is_ex = pkmn.is_ex;
    let is_mega = pkmn.is_mega;

    quote! {
        CardKind::Pokemon(PokemonCard {
            card_id: #card_id,
            base_id: #base_id,
            element_id: #element_id,
            stage_id: #stage_id,
            retreat_cost: #retreat_cost,
            hp: #hp,
            evolves_from_id: #evolves_from_id,
            flavor_text_id: #flavor_text_id,
            weakness_id: #weakness,
            ability_id: #ability,
            attack_ids: &[#(#attack_ids,)*],
            is_ex: #is_ex,
            is_mega: #is_mega,
        })
    }
}

fn gen_trainer_card(tr: &TrainerCard) -> TokenStream {
    let card_id = tr.card_id;
    let kind_id = tr.kind_id;
    let effect_id = tr.effect_id;

    quote! {
        CardKind::Trainer(TrainerCard {
            card_id: #card_id,
            kind_id: #kind_id,
            effect_id: #effect_id,
        })
    }
}

fn gen_card_versions(data: &Dataset) -> TokenStream {
    let cards = data.card_versions.iter().map(|card| {
        let id = card.id;
        let series_id = card.series_id;
        let set_id = card.set_id;
        let card_id = card.card_id;
        let packs_start = card.pack_ids.start;
        let packs_end = card.pack_ids.end;
        let number = card.number;
        let rarity_id = card.rarity_id;
        let illustrator_id = card.illustrator_id;
        let source_id = card.source_id;
        let is_foil = card.is_foil;
        let is_original = card.is_original;
        let is_tradable = card.is_tradable;
        let duplicate_ids = card.duplicate_ids.as_slice();
        let image_path = &card.image_path;

        quote! {
            CardVersion {
                id: #id,
                series_id: #series_id,
                set_id: #set_id,
                card_id: #card_id,
                pack_ids: #packs_start..#packs_end,
                number: std::num::NonZeroUsize::new(#number).unwrap(),
                rarity_id: #rarity_id,
                illustrator_id: #illustrator_id,
                source_id: #source_id,
                is_foil: #is_foil,
                is_original: #is_original,
                is_tradable: #is_tradable,
                duplicate_ids: &[#(#duplicate_ids,)*],
                #[cfg(feature = "images")]
                image: asset!(#image_path),
            }
        }
    });

    quote! {
        pub const CARD_VERSIONS: &[CardVersion] = &[#(#cards,)*];
    }
}

fn gen_card_sources(data: &Dataset) -> TokenStream {
    let sources = data.card_sources.iter().map(|source| {
        let id = source.id;
        let name_id = source.name_id;
        let description_id = source.description_id;
        let icon_path = &source.icon_path;

        quote! {
            CardSource {
                id: #id,
                name_id: #name_id,
                description_id: #description_id,
                #[cfg(feature = "images")]
                icon: asset!(#icon_path),
            }
        }
    });

    quote! {
        pub const CARD_SOURCES: &[CardSource] = &[#(#sources,)*];
    }
}

fn gen_elements(data: &Dataset) -> TokenStream {
    let elements = data.elements.iter().map(|elem| {
        let id = elem.id;
        let code = if let Some(code) = elem.code {
            quote! { Some(#code) }
        } else {
            quote! { None }
        };
        let name_id = elem.name_id;
        let icon_path = &elem.icon_path;
        let symbol_path = &elem.symbol_path;

        quote! {
            Element {
                id: #id,
                code: #code,
                name_id: #name_id,
                #[cfg(feature = "images")]
                icon: asset!(#icon_path),
                #[cfg(feature = "images")]
                symbol: asset!(#symbol_path),
            }
        }
    });

    quote! {
        pub const ELEMENTS: &[Element] = &[#(#elements,)*];
    }
}

fn gen_abilities(data: &Dataset) -> TokenStream {
    let abilities = data.abilities.iter().map(|ability| {
        let id = ability.id;
        let name_id = ability.name_id;
        let effect_id = ability.effect_id;

        quote! {
            Ability {
                id: #id,
                name_id: #name_id,
                effect_id: #effect_id,
            }
        }
    });

    quote! {
        pub const ABILITIES: &[Ability] = &[#(#abilities,)*];
    }
}

fn gen_attacks(data: &Dataset) -> TokenStream {
    let attacks = data.attacks.iter().map(|attack| {
        let id = attack.id;
        let name_id = attack.name_id;
        let effect_id = if let Some(id) = attack.effect_id {
            quote! { Some(#id) }
        } else {
            quote! { None }
        };
        let base_damage = attack.base_damage;
        let damage_suffix = if let Some(suff) = attack.damage_suffix {
            quote! { Some(#suff) }
        } else {
            quote! { None }
        };
        let cost_element_ids = attack.cost_element_ids.as_slice();

        quote! {
            Attack {
                id: #id,
                name_id: #name_id,
                effect_id: #effect_id,
                base_damage: #base_damage,
                damage_suffix: #damage_suffix,
                cost_element_ids: &[#(#cost_element_ids,)*],
            }
        }
    });

    quote! {
        pub const ATTACKS: &[Attack] = &[#(#attacks,)*];
    }
}

fn gen_base_pokemon(data: &Dataset) -> TokenStream {
    let base_pokemon = data.base_pokemon.iter().map(|pkmn| {
        let id = pkmn.id;
        let natdex_number = pkmn.natdex_number;
        let name_id = pkmn.name_id;

        quote! {
            BasePokemon {
                id: #id,
                natdex_num: std::num::NonZeroUsize::new(#natdex_number).unwrap(),
                name_id: #name_id,
            }
        }
    });

    quote! {
        pub const BASE_POKEMON: &[BasePokemon] = &[#(#base_pokemon,)*];
    }
}

fn gen_stages(data: &Dataset) -> TokenStream {
    let stages = data.stages.iter().map(|stage| {
        let id = stage.id;
        let name_id = stage.name_id;

        quote! {
            Stage {
                id: #id,
                name_id: #name_id,
            }
        }
    });

    quote! {
        pub const STAGES: &[Stage] = &[#(#stages,)*];
    }
}

fn gen_trainer_kinds(data: &Dataset) -> TokenStream {
    let kinds = data.trainer_kinds.iter().map(|kind| {
        let id = kind.id;
        let name_id = kind.name_id;

        quote! {
            TrainerKind {
                id: #id,
                name_id: #name_id,
            }
        }
    });

    quote! {
        pub const TRAINER_KINDS: &[TrainerKind] = &[#(#kinds,)*];
    }
}

fn gen_pack_variants(data: &Dataset) -> TokenStream {
    let variants = data.pack_variants.iter().map(|variant| {
        let id = variant.id;
        let name_id = variant.name_id;
        let series_id = variant.series_id;
        let set_id = variant.set_id;
        let pack_id = variant.pack_id;
        let pull_rate_num = variant.pull_rate.0;
        let pull_rate_den = variant.pull_rate.1;
        let slots_start = variant.slot_ids.start;
        let slots_end = variant.slot_ids.end;

        quote! {
            PackVariant {
                id: #id,
                name_id: #name_id,
                series_id: #series_id,
                set_id: #set_id,
                pack_id: #pack_id,
                pull_rate: Prob::new(#pull_rate_num, #pull_rate_den),
                slot_ids: #slots_start..#slots_end,
            }
        }
    });

    quote! {
        pub const PACK_VARIANTS: &[PackVariant] = &[#(#variants,)*];
    }
}

fn gen_pack_slots(data: &Dataset) -> TokenStream {
    let slots = data.pack_slots.iter().map(|slot| {
        let id = slot.id;
        let variant_id = slot.pack_variant_id;
        let pull_number = slot.pull_number;
        let rarity_pull_rates = slot.rarity_pull_rates.iter().map(|rate| {
            let rarity_id = rate.rarity_id;
            let normal_num = rate.normal.0;
            let normal_den = rate.normal.1;
            let foil_num = rate.foil.0;
            let foil_den = rate.foil.1;

            quote! {
                RarityPullRate {
                    rarity_id: #rarity_id,
                    normal: Prob::new(#normal_num, #normal_den),
                    foil: Prob::new(#foil_num, #foil_den),
                }
            }
        });
        let card_pull_rates = slot.card_pull_rates.iter().map(|card| {
            let card_version_id = card.card_version_id;
            let pull_rate_num = card.pull_rate.0;
            let pull_rate_den = card.pull_rate.1;

            quote! {
                CardVersionPullRate {
                    card_version_id: #card_version_id,
                    pull_rate: Prob::new(#pull_rate_num, #pull_rate_den),
                }
            }
        });

        quote! {
            PackSlot {
                id: #id,
                variant_id: #variant_id,
                pull_number: #pull_number,
                rarities: &[#(#rarity_pull_rates,)*],
                card_versions: &[#(#card_pull_rates,)*],
            }
        }
    });

    quote! {
        pub const PACK_SLOTS: &[PackSlot] = &[#(#slots,)*];
    }
}

fn gen_date(date: NaiveDate) -> TokenStream {
    let y = date.year();
    let m = date.month();
    let d = date.day();

    quote! {
        chrono::NaiveDate::from_ymd_opt(#y, #m, #d)
    }
}

fn gen_date_opt(date: Option<NaiveDate>) -> TokenStream {
    if let Some(date) = date {
        gen_date(date)
    } else {
        quote! { None }
    }
}
