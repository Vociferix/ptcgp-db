use crate::load::{self, RawData};
use crate::models::{self, Dataset};

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;

use std::collections::hash_map::{Entry, HashMap};

struct State {
    raw_data: RawData,
    data: Dataset,
    lookups: Lookups,
}

#[derive(Default)]
struct Lookups {
    rarities: HashMap<String, usize>,
    elements: HashMap<String, usize>,
    card_sources: HashMap<String, usize>,
    pack_variant_names: HashMap<String, usize>,
    card_ids: HashMap<usize, usize>,
    base_pokemon: HashMap<usize, usize>,
    stages: HashMap<String, usize>,
    trainer_kinds: HashMap<String, usize>,
    series: HashMap<String, usize>,
    sets: HashMap<String, usize>,
    // { set_id -> { pack_subtitle -> pack_id } }
    packs: HashMap<usize, HashMap<String, usize>>,
    // { set_id -> { number -> card_version_id } }
    card_versions: HashMap<usize, HashMap<usize, usize>>,
    // { pack_id -> { variant_name -> pack_variant_id } }
    pack_variants: HashMap<usize, HashMap<String, usize>>,
}

struct OptOrd<T>(Option<T>);

impl<T: PartialEq> PartialEq for OptOrd<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Eq> Eq for OptOrd<T> {}

impl<T: PartialOrd> PartialOrd for OptOrd<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (&self.0, &other.0) {
            (None, None) => Some(std::cmp::Ordering::Equal),
            (Some(_), None) => Some(std::cmp::Ordering::Less),
            (None, Some(_)) => Some(std::cmp::Ordering::Greater),
            (Some(l), Some(r)) => PartialOrd::partial_cmp(l, r),
        }
    }
}

impl<T: Ord> Ord for OptOrd<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self.0, &other.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(l), Some(r)) => Ord::cmp(l, r),
        }
    }
}

pub fn marshal(raw_data: RawData) -> Result<Dataset> {
    let mut state = State {
        raw_data,
        data: Dataset::default(),
        lookups: Lookups::default(),
    };

    collect_strings(&mut state);
    elements(&mut state).context("failed to marshal elements")?;
    card_sources(&mut state).context("failed to marshal card sources")?;
    rarities(&mut state).context("failed to marshal rarities")?;
    base_pokemon(&mut state).context("failed to marshal base pokemon")?;
    series(&mut state).context("failed to marshal series")?;
    sets(&mut state).context("failed to marshal sets")?;
    packs(&mut state).context("failed to marshal packs")?;
    card_versions(&mut state).context("failed to marshal card versions")?;
    stages(&mut state).context("failed to marshal stages")?;
    trainer_kinds(&mut state).context("failed to marshal trainer kinds")?;
    cards(&mut state).context("failed to marshal cards")?;
    pack_variants(&mut state).context("failed to marshal pack variants")?;
    pack_slots(&mut state).context("failed to marshal pack slots")?;

    Ok(std::mem::take(&mut state.data))
}

fn elements(state: &mut State) -> Result<()> {
    for (id, elem) in state.raw_data.elements.iter().enumerate() {
        let name_id = str_id(&state.data.element_names, &elem.name)
            .context("failed to resolve element name")?;
        let slug = make_slug(&elem.name);
        let icon_path = format!("ptcgp-images/elements/icons/{slug}.png");
        let symbol_path = format!("ptcgp-images/elements/symbols/{slug}.png");
        state.data.elements.push(models::Element {
            id,
            code: elem.symbol,
            name_id,
            icon_path,
            symbol_path,
        });
        if state
            .lookups
            .elements
            .insert(elem.name.clone(), id)
            .is_some()
        {
            bail!("duplicate element name: {:?}", elem.name);
        }
    }

    Ok(())
}

fn card_sources(state: &mut State) -> Result<()> {
    for (id, src) in state.raw_data.card_sources.iter().enumerate() {
        let name_id = str_id(&state.data.card_source_names, &src.code)
            .context("failed to resolve card source name")?;
        let description_id = str_id(&state.data.card_source_descriptions, &src.description)
            .context("failed to resolve card source description")?;
        let slug = make_slug(&src.code);
        let icon_path = format!("ptcgp-images/card_sources/{slug}.png");

        state.data.card_sources.push(models::CardSource {
            id,
            name_id,
            description_id,
            icon_path,
        });

        state.lookups.card_sources.insert(src.code.clone(), id);
    }

    Ok(())
}

fn rarities(state: &mut State) -> Result<()> {
    let mut groups: HashMap<String, usize> = HashMap::new();
    let mut classes: HashMap<(String, usize), usize> = HashMap::new();

    for (id, rarity) in state.raw_data.rarities.iter().enumerate() {
        let count = rarity.group_symbol_count;

        let mut group_id = state.data.rarity_groups.len();
        match groups.entry(rarity.group.clone()) {
            Entry::Vacant(entry) => {
                let name_id = str_id(&state.data.rarity_group_names, &rarity.group)
                    .context("failed to resolve rarity group name")?;
                let slug = make_slug(&rarity.group);
                let icon_path = format!("ptcgp-images/rarities/icons/{slug}/1.png");
                let symbol_path = format!("ptcgp-images/rarities/symbols/{slug}/1.png");
                state.data.rarity_groups.push(models::RarityGroup {
                    id: group_id,
                    name_id,
                    icon_path,
                    symbol_path,
                });
                entry.insert(group_id);
            }
            Entry::Occupied(entry) => {
                group_id = *entry.get();
            }
        }

        let mut class_id = state.data.rarity_classes.len();
        match classes.entry((rarity.group.clone(), count)) {
            Entry::Vacant(entry) => {
                let slug = make_slug(&rarity.group);
                let icon_path = format!("ptcgp-images/rarities/icons/{slug}/{count}.png");
                let symbol_path = format!("ptcgp-images/rarities/symbols/{slug}/{count}.png");
                state.data.rarity_classes.push(models::RarityClass {
                    id: class_id,
                    group_id,
                    count,
                    icon_path,
                    symbol_path,
                });
                entry.insert(class_id);
            }
            Entry::Occupied(entry) => {
                class_id = *entry.get();
            }
        }

        let code_id = str_id(&state.data.rarity_codes, &rarity.code)
            .context("failed to resolve rarity code")?;
        let name_id = str_id(&state.data.rarity_names, &rarity.name)
            .context("failed to resolve rarity name")?;

        state.data.rarities.push(models::Rarity {
            id,
            group_id,
            class_id,
            code_id,
            name_id,
            craft_cost: rarity.craft_cost,
            dupe_dust: rarity.dupe_dust,
        });

        state.lookups.rarities.insert(rarity.code.clone(), id);
    }

    Ok(())
}

fn base_pokemon(state: &mut State) -> Result<()> {
    for (id, pkmn) in state.raw_data.base_pokemon.iter().enumerate() {
        state.lookups.base_pokemon.insert(pkmn.natdex_number, id);
        state.data.base_pokemon.push(models::BasePokemon {
            id,
            natdex_number: pkmn.natdex_number,
            name_id: str_id(&state.data.base_pokemon_names, &pkmn.name)
                .context("failed to resolve base pokemon name")?,
        });
    }

    Ok(())
}

fn series(state: &mut State) -> Result<()> {
    let mut series: HashMap<usize, (models::Series, Option<NaiveDate>)> = HashMap::new();
    for set in &state.raw_data.sets {
        let code_id = str_id(&state.data.series_codes, &set.series)
            .context("failed to resolve series code")?;

        match series.entry(code_id) {
            Entry::Vacant(entry) => {
                entry.insert((
                    models::Series {
                        id: 0,
                        code_id,
                        set_ids: 0..0,
                        pack_ids: 0..0,
                        card_version_ids: 0..0,
                    },
                    set.availability.map(|avail| avail.start),
                ));
            }
            Entry::Occupied(mut entry) => {
                if let Some(start) = set.availability.map(|avail| avail.start)
                    && entry.get().1.is_none_or(|old| old > start)
                {
                    entry.get_mut().1 = Some(start);
                }
            }
        }
    }

    let mut series: Vec<(models::Series, Option<NaiveDate>)> = series.into_values().collect();
    series.sort_by(|(l_series, l_start), (r_series, r_start)| {
        Ord::cmp(
            &(OptOrd(*l_start), l_series.code_id),
            &(OptOrd(*r_start), r_series.code_id),
        )
    });

    let mut series: Vec<models::Series> = series.into_iter().map(|(series, _)| series).collect();
    series.iter_mut().enumerate().for_each(|(id, series)| {
        series.id = id;
        state
            .lookups
            .series
            .insert(state.data.series_codes[series.code_id].clone(), id);
    });

    state.data.series = series;

    Ok(())
}

fn sets(state: &mut State) -> Result<()> {
    let mut sets: HashMap<usize, models::Set> = HashMap::new();
    for set in &state.raw_data.sets {
        let code_id =
            str_id(&state.data.set_codes, &set.code).context("failed to resolve set code")?;
        let name_id =
            str_id(&state.data.set_names, &set.name).context("failed to resolve set name")?;
        let series_id = *state
            .lookups
            .series
            .get(&set.series)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve series {:?}", set.series))?;
        let (release_date, retirement_date) = set
            .availability
            .map_or((None, None), |avail| (Some(avail.start), avail.end));
        let is_promo = set.is_promo;
        let logo_path = format!("ptcgp-images/sets/logos/{}.png", set.code);
        let icon_path = format!("ptcgp-images/sets/icons/{}.png", set.code);

        let Entry::Vacant(entry) = sets.entry(code_id) else {
            bail!("duplicate set with code {:?}", set.code);
        };

        entry.insert(models::Set {
            id: 0,
            series_id,
            code_id,
            name_id,
            release_date,
            retirement_date,
            is_promo,
            pack_ids: 0..0,
            card_version_ids: 0..0,
            logo_path,
            icon_path,
        });
    }

    let mut sets: Vec<models::Set> = sets.into_values().collect();
    sets.sort_by(|l, r| {
        Ord::cmp(
            &(l.series_id, OptOrd(l.release_date), l.code_id),
            &(r.series_id, OptOrd(r.release_date), r.code_id),
        )
    });

    sets.iter_mut().enumerate().for_each(|(id, set)| {
        set.id = id;
        state
            .lookups
            .sets
            .insert(state.data.set_codes[set.code_id].clone(), id);
    });

    let mut start = 0usize;
    for series in &mut state.data.series {
        let len = sets[start..]
            .iter()
            .position(|set| set.series_id != series.id)
            .unwrap_or_else(|| sets.len() - start);
        let end = start + len;
        series.set_ids = start..end;
        start = end;
    }

    state.data.sets = sets;

    Ok(())
}

fn packs(state: &mut State) -> Result<()> {
    let mut packs: Vec<models::Pack> = Vec::new();
    for set in &state.raw_data.sets {
        let Some(set_id) = state.lookups.sets.get(&set.code).copied() else {
            bail!("failed resolve set {:?}", set.code);
        };

        let set_model = &state.data.sets[set_id];

        for pack in &set.packs {
            let subtitle_id = str_id(&state.data.pack_subtitles, &pack)
                .context("failed to resolve pack subtitle")?;

            let slug = pack.to_lowercase().replace(" ", "_");

            let image_path = format!("ptcgp-images/packs/art/{}/{}.png", set.code, slug);
            let logo_path = format!("ptcgp-images/packs/logos/{}/{}.png", set.code, slug);

            packs.push(models::Pack {
                id: 0,
                series_id: set_model.series_id,
                set_id: set_model.id,
                subtitle_id,
                card_version_ids: Vec::new(),
                variant_ids: 0..0,
                image_path,
                logo_path,
            });
        }
    }

    packs.sort_by(|l, r| Ord::cmp(&(l.set_id, l.subtitle_id), &(r.set_id, r.subtitle_id)));

    packs.iter_mut().enumerate().for_each(|(id, pack)| {
        pack.id = id;
        state
            .lookups
            .packs
            .entry(pack.set_id)
            .or_default()
            .insert(state.data.pack_subtitles[pack.subtitle_id].clone(), id);
    });

    let mut start = 0usize;
    for series in &mut state.data.series {
        let len = packs[start..]
            .iter()
            .position(|pack| pack.series_id != series.id)
            .unwrap_or_else(|| packs.len() - start);
        let end = start + len;
        series.pack_ids = start..end;
        start = end;
    }

    start = 0;
    for set in &mut state.data.sets {
        let len = packs[start..]
            .iter()
            .position(|pack| pack.set_id != set.id)
            .unwrap_or_else(|| packs.len() - start);
        let end = start + len;
        set.pack_ids = start..end;
        start = end;
    }

    state.data.packs = packs;

    Ok(())
}

fn card_versions(state: &mut State) -> Result<()> {
    let mut card_versions: Vec<models::CardVersion> = Vec::new();
    for card_ver in &state.raw_data.card_versions {
        let set_id =
            *state.lookups.sets.get(&card_ver.set).ok_or_else(|| {
                anyhow::anyhow!("failed to resolve set with code {:?}", card_ver.set)
            })?;
        let illustrator_id = card_ver
            .illustrator
            .as_ref()
            .map(|illustrator| {
                str_id(&state.data.illustrators, illustrator)
                    .context("failed to resolve illustrator")
            })
            .transpose()?;
        let source_id = *state
            .lookups
            .card_sources
            .get(&card_ver.source)
            .ok_or_else(|| {
                anyhow::anyhow!("failed to resolve card source {:?}", card_ver.source)
            })?;
        let rarity_id = *state
            .lookups
            .rarities
            .get(&card_ver.rarity)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve rarity {:?}", card_ver.rarity))?;
        let set_code = state.data.set_codes[state.data.sets[set_id].code_id].as_str();
        let image_path = format!("ptcgp-images/cards/{}/{:03}.png", set_code, card_ver.number);

        let pack_ids = state.data.sets[set_id].pack_ids.clone();
        let pack_ids = if card_ver.packs.is_empty() {
            0usize..0
        } else if card_ver.packs.len() == 1 {
            let id = *state
                .lookups
                .packs
                .get(&set_id)
                .and_then(|map| map.get(&card_ver.packs[0]))
                .ok_or_else(|| anyhow::anyhow!("failed to resolve pack {:?}", card_ver.packs[0]))?;
            id..(id + 1)
        } else if card_ver.packs.len() == pack_ids.end - pack_ids.start {
            pack_ids
        } else {
            bail!(
                "failed to resolve packs for {}-{:03}",
                state.data.set_codes[state.data.sets[set_id].code_id],
                card_ver.number
            );
        };

        card_versions.push(models::CardVersion {
            id: 0,
            series_id: state.data.sets[set_id].series_id,
            set_id,
            card_id: card_ver.card_id,
            pack_ids,
            number: card_ver.number,
            rarity_id,
            illustrator_id,
            source_id,
            is_foil: card_ver.is_foil,
            is_original: !card_ver.is_reprint,
            is_tradable: card_ver.is_tradable,
            duplicate_ids: Vec::new(),
            image_path,
        });
    }

    card_versions.sort_by(|l, r| Ord::cmp(&(l.set_id, l.number), &(r.set_id, r.number)));

    card_versions
        .iter_mut()
        .enumerate()
        .for_each(|(id, card_ver)| {
            card_ver.id = id;
            match state.lookups.card_versions.entry(card_ver.set_id) {
                Entry::Vacant(entry) => {
                    entry.insert(HashMap::new()).insert(card_ver.number, id);
                }
                Entry::Occupied(mut entry) => {
                    entry.get_mut().insert(card_ver.number, id);
                }
            }

            for pack_id in card_ver.pack_ids.clone() {
                state.data.packs[pack_id].card_version_ids.push(id);
            }
        });

    for card_ver in &state.raw_data.card_versions {
        let mut duplicates: Vec<usize> = Vec::with_capacity(card_ver.duplicates.len());
        for cvref in &card_ver.duplicates {
            let set_id = *state
                .lookups
                .sets
                .get(&cvref.set)
                .ok_or_else(|| anyhow::anyhow!("failed to resolve set {:?}", cvref.set))?;
            let id = *state
                .lookups
                .card_versions
                .get(&set_id)
                .ok_or_else(|| anyhow::anyhow!("failed to resolve set {:?}", cvref.set))?
                .get(&cvref.number)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "failed to resolve card version {}-{:03}",
                        cvref.set,
                        cvref.number
                    )
                })?;
            duplicates.push(id);
        }
        duplicates.sort();

        let set_id = *state
            .lookups
            .sets
            .get(&card_ver.set)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve set {:?}", card_ver.set))?;
        let id = *state
            .lookups
            .card_versions
            .get(&set_id)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve set {:?}", card_ver.set))?
            .get(&card_ver.number)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "failed to resolve card version {}-{:03}",
                    card_ver.set,
                    card_ver.number
                )
            })?;
        card_versions[id].duplicate_ids = duplicates;
    }

    let mut start = 0usize;
    for series in &mut state.data.series {
        let len = card_versions[start..]
            .iter()
            .position(|cv| cv.series_id != series.id)
            .unwrap_or_else(|| card_versions.len() - start);
        let end = start + len;
        series.card_version_ids = start..end;
        start = end;
    }

    start = 0usize;
    for set in &mut state.data.sets {
        let len = card_versions[start..]
            .iter()
            .position(|cv| cv.series_id != set.id)
            .unwrap_or_else(|| card_versions.len() - start);
        let end = start + len;
        set.card_version_ids = start..end;
        start = end;
    }

    state.data.card_versions = card_versions;

    Ok(())
}

fn stages(state: &mut State) -> Result<()> {
    for id in 0..state.data.stage_names.len() {
        state.data.stages.push(models::Stage { id, name_id: id });
        state
            .lookups
            .stages
            .insert(state.data.stage_names[id].clone(), id);
    }
    Ok(())
}

fn trainer_kinds(state: &mut State) -> Result<()> {
    for id in 0..state.data.trainer_kind_names.len() {
        state
            .data
            .trainer_kinds
            .push(models::TrainerKind { id, name_id: id });
        state
            .lookups
            .trainer_kinds
            .insert(state.data.trainer_kind_names[id].clone(), id);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AbilityKey {
    name_id: usize,
    effect_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AttackKey {
    name_id: usize,
    effect_id: Option<usize>,
    base_damage: u32,
    damage_suffix: Option<char>,
    cost: Vec<usize>,
}

fn cards(state: &mut State) -> Result<()> {
    let mut abilities: Vec<models::Ability> = Vec::new();
    let mut ability_table: HashMap<AbilityKey, usize> = HashMap::new();
    let mut attacks: Vec<models::Attack> = Vec::new();
    let mut attack_table: HashMap<AttackKey, usize> = HashMap::new();
    let mut cards: Vec<models::Card> = Vec::new();

    for card_ver in &mut state.data.card_versions {
        if let Some(card_id) = state.lookups.card_ids.get(&card_ver.card_id).copied() {
            card_ver.card_id = card_id;
            cards[card_id].version_ids.push(card_ver.id);
            continue;
        }
        let card = state
            .raw_data
            .cards
            .get(&card_ver.card_id)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve card {}", card_ver.card_id))?;
        let id = cards.len();
        state.lookups.card_ids.insert(card_ver.card_id, id);
        card_ver.card_id = id;

        let name_id =
            str_id(&state.data.card_names, &card.name).context("failed to resolve card name")?;

        match &card.kind {
            load::CardKind::Pokemon { pokemon: pkmn } => {
                let base_id = *state
                    .lookups
                    .base_pokemon
                    .get(&pkmn.natdex_number)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "failed to resolve base pokemon national dex number {}",
                            pkmn.natdex_number
                        )
                    })?;

                let element_id = *state.lookups.elements.get(&pkmn.element).ok_or_else(|| {
                    anyhow::anyhow!("failed to resolve element {:?}", pkmn.element)
                })?;

                let stage_id = *state.lookups.stages.get(&pkmn.stage).ok_or_else(|| {
                    anyhow::anyhow!("failed to resolve pokemon card stage {:?}", pkmn.stage)
                })?;

                let flavor_text_id = pkmn
                    .flavor
                    .as_ref()
                    .map(|flavor| {
                        str_id(&state.data.flavor_text, flavor)
                            .context("failed to resolve flavor text")
                    })
                    .transpose()?;

                let evolves_from_id = pkmn
                    .evolves_from
                    .as_ref()
                    .map(|evolves_from| {
                        str_id(&state.data.card_names, evolves_from)
                            .context("failed to resolve pokemon card 'evolves from' name")
                    })
                    .transpose()?;

                let weakness_id =
                    pkmn.weakness
                        .as_ref()
                        .map(|element| {
                            state.lookups.elements.get(element).copied().ok_or_else(|| {
                                anyhow::anyhow!("failed to resolve element {element:?}")
                            })
                        })
                        .transpose()?;

                let ability_id = pkmn
                    .ability
                    .as_ref()
                    .map(|ability| -> Result<usize> {
                        let name_id = str_id(&state.data.ability_names, &ability.name)
                            .context("failed to resolve ability name")?;
                        let effect_id = str_id(&state.data.ability_effects, &ability.effect)
                            .context("failed to resolve ability effect")?;
                        let key = AbilityKey { name_id, effect_id };
                        Ok(match ability_table.entry(key) {
                            Entry::Vacant(entry) => {
                                let id = abilities.len();
                                abilities.push(models::Ability {
                                    id,
                                    name_id,
                                    effect_id,
                                });
                                entry.insert(id);
                                id
                            }
                            Entry::Occupied(entry) => *entry.get(),
                        })
                    })
                    .transpose()?;

                let attack_ids = pkmn.attacks.iter().map(|attack| -> Result<usize> {
                    let name_id = str_id(&state.data.attack_names, &attack.name)
                        .context("failed to resolve attack name")?;

                    let effect_id = attack.effect.as_ref().map(|effect| {
                        str_id(&state.data.attack_effects, effect)
                            .context("failed to resolve attack effect")
                    }).transpose()?;

                    let suffix = attack.damage_suffix.as_ref().map(|suffix| -> Result<char> {
                        let mut chars = suffix.chars();
                        let Some(ch) = chars.next() else {
                            bail!("damage suffix for attack {:?} is not null but is empty", attack.name);
                        };
                        if chars.next().is_some() {
                            bail!("damage suffix for attack {:?} is more than a single Unicode codepoint", attack.name);
                        }
                        Ok(ch)
                    }).transpose()?;

                    let cost_element_ids = attack.cost.iter().map(|element| {
                        state.lookups.elements.get(element)
                            .ok_or_else(|| anyhow::anyhow!("failed to resolve element {element:?}"))
                            .copied()
                    }).collect::<Result<Vec<usize>>>()?;

                    let key = AttackKey {
                        name_id,
                        effect_id,
                        base_damage: attack.damage,
                        damage_suffix: suffix,
                        cost: cost_element_ids.clone(),
                    };

                    Ok(match attack_table.entry(key) {
                        Entry::Vacant(entry) => {
                            let id = attacks.len();
                            attacks.push(models::Attack {
                                id,
                                name_id,
                                effect_id,
                                base_damage: attack.damage,
                                damage_suffix: suffix,
                                cost_element_ids,
                            });
                            entry.insert(id);
                            id
                        },
                        Entry::Occupied(entry) => {
                            *entry.get()
                        },
                    })
                }).collect::<Result<Vec<usize>>>()?;

                cards.push(models::Card {
                    id,
                    name_id,
                    version_ids: vec![card_ver.id],
                    kind: models::CardKind::Pokemon(models::PokemonCard {
                        card_id: id,
                        base_id,
                        element_id,
                        stage_id,
                        retreat_cost: pkmn.retreat_cost,
                        hp: pkmn.hp,
                        evolves_from_id,
                        flavor_text_id,
                        weakness_id,
                        ability_id,
                        attack_ids,
                        is_ex: pkmn.is_ex,
                        is_mega: pkmn.is_mega,
                    }),
                });
            }
            load::CardKind::Trainer { trainer: tr } => {
                let kind_id = *state.lookups.trainer_kinds.get(&tr.kind).ok_or_else(|| {
                    anyhow::anyhow!("failed to resolve trainer kind {:?}", tr.kind)
                })?;
                let effect_id = str_id(&state.data.trainer_effects, &tr.effect)
                    .context("failed to resolve trainer effect")?;

                cards.push(models::Card {
                    id,
                    name_id,
                    version_ids: vec![card_ver.id],
                    kind: models::CardKind::Trainer(models::TrainerCard {
                        card_id: id,
                        kind_id,
                        effect_id,
                    }),
                });
            }
        }
    }

    state.data.cards = cards;
    state.data.attacks = attacks;
    state.data.abilities = abilities;

    Ok(())
}

fn pack_variants(state: &mut State) -> Result<()> {
    let mut pack_variants: Vec<models::PackVariant> = Vec::new();
    for pack in &state.raw_data.pack_data {
        let set_id = *state
            .lookups
            .sets
            .get(&pack.set)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve set {:?}", pack.set))?;

        let pack_id = *state
            .lookups
            .packs
            .get(&set_id)
            .and_then(|map| map.get(&pack.subtitle))
            .ok_or_else(|| anyhow::anyhow!("failed to resolve pack {:?}", pack.subtitle))?;

        if pack.set
            != state.data.set_codes[state.data.sets[state.data.packs[pack_id].set_id].code_id]
        {
            bail!(
                "pack data entry mismatch: pack {:?} is not in set {:?}",
                pack.subtitle,
                pack.set
            );
        }

        for (code, variant) in &pack.variants {
            let pack = &state.data.packs[pack_id];

            let name_id = *state
                .lookups
                .pack_variant_names
                .get(code)
                .ok_or_else(|| anyhow::anyhow!("failed to resolve pack variant name"))?;

            let rate = simplify(variant.rate.numerator, variant.rate.denominator);

            pack_variants.push(models::PackVariant {
                id: 0,
                name_id,
                series_id: pack.series_id,
                set_id: pack.set_id,
                pack_id,
                pull_rate: rate,
                slot_ids: 0..0,
            });
        }
    }

    pack_variants.sort_by(|l, r| {
        let l_rate = l.pull_rate.0 as u128 * r.pull_rate.1 as u128;
        let r_rate = r.pull_rate.0 as u128 * l.pull_rate.1 as u128;
        Ord::cmp(&(l.pack_id, l_rate), &(r.pack_id, r_rate))
    });

    pack_variants
        .iter_mut()
        .enumerate()
        .for_each(|(id, variant)| {
            variant.id = id;
            match state.lookups.pack_variants.entry(variant.pack_id) {
                Entry::Vacant(entry) => {
                    entry
                        .insert(HashMap::new())
                        .insert(state.data.pack_variant_names[variant.name_id].clone(), id);
                }
                Entry::Occupied(mut entry) => {
                    entry
                        .get_mut()
                        .insert(state.data.pack_variant_names[variant.name_id].clone(), id);
                }
            }
        });

    let mut start = 0usize;
    for pack in &mut state.data.packs {
        let len = pack_variants[start..]
            .iter()
            .position(|pv| pv.series_id != pack.id)
            .unwrap_or_else(|| pack_variants.len() - start);
        let end = start + len;
        pack.variant_ids = start..end;
        start = end;
    }

    state.data.pack_variants = pack_variants;

    Ok(())
}

fn pack_slots(state: &mut State) -> Result<()> {
    let mut slots: Vec<models::PackSlot> = Vec::new();

    for pack in &state.raw_data.pack_data {
        let set_id = *state
            .lookups
            .sets
            .get(&pack.set)
            .ok_or_else(|| anyhow::anyhow!("failed to resolve set {:?}", pack.set))?;

        let pack_id = *state
            .lookups
            .packs
            .get(&set_id)
            .and_then(|map| map.get(&pack.subtitle))
            .ok_or_else(|| anyhow::anyhow!("failed to resolve pack {:?}", pack.subtitle))?;

        for (code, variant) in &pack.variants {
            let pack = &state.data.packs[pack_id];

            let name_id = *state
                .lookups
                .pack_variant_names
                .get(code)
                .ok_or_else(|| anyhow::anyhow!("failed to resolve pack variant name"))?;

            let variant_id = *state
                .lookups
                .pack_variants
                .get(&pack_id)
                .and_then(|map| map.get(&state.data.pack_variant_names[name_id]))
                .ok_or_else(|| anyhow::anyhow!("failed to resolve pack variant"))?;

            for slot_idx in 0..variant.slot_count {
                let mut rarity_pull_rates: Vec<models::RarityPullRate> = Vec::new();
                for (rarity_code, rates) in &variant.rarity_rates_by_slot[slot_idx] {
                    let rarity_id = *state.lookups.rarities.get(rarity_code).ok_or_else(|| {
                        anyhow::anyhow!("failed to resolve rarity {:?}", rarity_code)
                    })?;

                    rarity_pull_rates.push(models::RarityPullRate {
                        rarity_id,
                        normal: simplify(rates.normal.numerator, rates.normal.denominator),
                        foil: simplify(rates.foil.numerator, rates.normal.denominator),
                    });
                }
                rarity_pull_rates.sort_by(|l, r| Ord::cmp(&l.rarity_id, &r.rarity_id));

                let mut card_pull_rates: Vec<models::CardVersionPullRate> = Vec::new();
                for (number, rates) in &variant.card_rates {
                    if let Some(rate) = &rates[slot_idx] {
                        let number: usize = number
                            .parse()
                            .context("invalid card version number in pull rates")?;
                        let card_version_id = *state
                            .lookups
                            .card_versions
                            .get(&pack.set_id)
                            .and_then(|map| map.get(&number))
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "failed to resolve card {} for pack {:?}",
                                    number,
                                    state.data.pack_subtitles[pack.subtitle_id]
                                )
                            })?;

                        card_pull_rates.push(models::CardVersionPullRate {
                            card_version_id,
                            pull_rate: simplify(rate.numerator, rate.denominator),
                        });
                    }
                }
                card_pull_rates.sort_by(|l, r| Ord::cmp(&l.card_version_id, &r.card_version_id));

                slots.push(models::PackSlot {
                    id: 0,
                    pack_variant_id: variant_id,
                    pull_number: slot_idx,
                    rarity_pull_rates,
                    card_pull_rates,
                });
            }
        }
    }

    slots.sort_by(|l, r| {
        Ord::cmp(
            &(l.pack_variant_id, l.pull_number),
            &(r.pack_variant_id, r.pull_number),
        )
    });

    slots.iter_mut().enumerate().for_each(|(id, slot)| {
        slot.id = id;
    });

    let mut start = 0usize;
    for variant in &mut state.data.pack_variants {
        let len = slots[start..]
            .iter()
            .position(|slot| slot.pack_variant_id != variant.id)
            .unwrap_or_else(|| slots.len() - start);
        let end = start + len;
        variant.slot_ids = start..end;
        start = end;
    }

    state.data.pack_slots = slots;

    Ok(())
}

fn simplify(num: u64, den: u64) -> (u64, u64) {
    let gcd = gcd::binary_u64(num, den);
    (num / gcd, den / gcd)
}

fn collect_strings(state: &mut State) {
    collect_rarity_strings(&state.raw_data, &mut state.data);
    collect_set_strings(&state.raw_data, &mut state.data);
    collect_card_strings(&state.raw_data, &mut state.data);
    collect_card_version_strings(&state.raw_data, &mut state.data);
    collect_base_pokemon_strings(&state.raw_data, &mut state.data);
    collect_card_source_strings(&state.raw_data, &mut state.data);
    collect_element_strings(&state.raw_data, &mut state.data);
    collect_pack_variant_strings(&state.raw_data, &mut state.data, &mut state.lookups);
}

fn make_str_table(strings: &mut Vec<String>) {
    strings.sort();
    strings.dedup();
    strings.shrink_to_fit();
}

fn collect_rarity_strings(raw_data: &RawData, data: &mut Dataset) {
    for rarity in &raw_data.rarities {
        data.rarity_codes.push(rarity.code.clone());
        data.rarity_names.push(rarity.name.clone());
        data.rarity_group_names.push(rarity.group.clone());
    }

    make_str_table(&mut data.rarity_codes);
    make_str_table(&mut data.rarity_names);
    make_str_table(&mut data.rarity_group_names);
}

fn collect_set_strings(raw_data: &RawData, data: &mut Dataset) {
    for set in &raw_data.sets {
        data.series_codes.push(set.series.clone());
        data.set_codes.push(set.code.clone());
        data.set_names.push(set.name.clone());
        data.pack_subtitles.extend(set.packs.iter().cloned());
    }

    make_str_table(&mut data.series_codes);
    make_str_table(&mut data.set_codes);
    make_str_table(&mut data.set_names);
    make_str_table(&mut data.pack_subtitles);
}

fn collect_card_strings(raw_data: &RawData, data: &mut Dataset) {
    for card in raw_data.cards.values() {
        data.card_names.push(card.name.clone());

        match &card.kind {
            load::CardKind::Pokemon { pokemon } => {
                data.stage_names.push(pokemon.stage.clone());
                if let Some(flavor) = &pokemon.flavor {
                    data.flavor_text.push(flavor.clone());
                }
                if let Some(ability) = &pokemon.ability {
                    data.ability_names.push(ability.name.clone());
                    data.ability_effects.push(ability.effect.clone());
                }
                for attack in &pokemon.attacks {
                    data.attack_names.push(attack.name.clone());
                    if let Some(effect) = &attack.effect {
                        data.attack_effects.push(effect.clone());
                    }
                }
            }
            load::CardKind::Trainer { trainer } => {
                data.trainer_kind_names.push(trainer.kind.clone());
                data.trainer_effects.push(trainer.effect.clone());
            }
        }
    }

    make_str_table(&mut data.card_names);
    make_str_table(&mut data.stage_names);
    make_str_table(&mut data.flavor_text);
    make_str_table(&mut data.ability_names);
    make_str_table(&mut data.ability_effects);
    make_str_table(&mut data.attack_names);
    make_str_table(&mut data.attack_effects);
    make_str_table(&mut data.trainer_kind_names);
    make_str_table(&mut data.trainer_effects);
}

fn collect_card_version_strings(raw_data: &RawData, data: &mut Dataset) {
    for card in &raw_data.card_versions {
        if let Some(illustrator) = &card.illustrator {
            data.illustrators.push(illustrator.clone());
        }
    }

    make_str_table(&mut data.illustrators);
}

fn collect_base_pokemon_strings(raw_data: &RawData, data: &mut Dataset) {
    for pkmn in &raw_data.base_pokemon {
        data.base_pokemon_names.push(pkmn.name.clone());
    }

    make_str_table(&mut data.base_pokemon_names);
}

fn collect_card_source_strings(raw_data: &RawData, data: &mut Dataset) {
    for src in &raw_data.card_sources {
        data.card_source_names.push(src.code.clone());
        data.card_source_descriptions.push(src.description.clone());
    }

    make_str_table(&mut data.card_source_names);
    make_str_table(&mut data.card_source_descriptions);
}

fn collect_element_strings(raw_data: &RawData, data: &mut Dataset) {
    for elem in &raw_data.elements {
        data.element_names.push(elem.name.clone());
    }

    make_str_table(&mut data.element_names);
}

fn collect_pack_variant_strings(raw_data: &RawData, data: &mut Dataset, lookups: &mut Lookups) {
    for variant in &raw_data.pack_variant_names {
        data.pack_variant_names.push(variant.name.clone());
    }

    make_str_table(&mut data.pack_variant_names);

    for variant in &raw_data.pack_variant_names {
        if let Ok(id) = str_id(&data.pack_variant_names, &variant.name) {
            lookups.pack_variant_names.insert(variant.code.clone(), id);
        }
    }
}

fn str_id(table: &[String], s: &str) -> Result<usize> {
    table
        .binary_search_by(|k| Ord::cmp(k.as_str(), s))
        .map_err(|_| anyhow::anyhow!("string {s:?} not in string table"))
}

fn make_slug(s: &str) -> String {
    s.to_lowercase().replace(" ", "_")
}
