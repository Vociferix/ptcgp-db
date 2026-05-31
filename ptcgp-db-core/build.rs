use ptcgp_db_data::{CardVersion, Pack, Prob};

fn main() {
    let out_dir: std::path::PathBuf = std::env::var_os("OUT_DIR")
        .expect("OUT_DIR env variable")
        .into();
    let out_path = out_dir.join("generated.rs");
    let mut out = std::io::BufWriter::new(
        std::fs::File::create(out_path).expect("create $OUT_DIR/generated.rs"),
    );
    generate(&mut out).expect("write generated code");
}

fn generate(out: &mut impl std::io::Write) -> std::io::Result<()> {
    let mut pull_rates = Vec::with_capacity(3);

    writeln!(out, "pub struct CardPullRates {{")?;
    writeln!(out, "    pub max_pull_rate_pct: f64,")?;
    writeln!(out, "    pub max_pull_rate: Prob,")?;
    writeln!(out, "    pub best_pack: Option<&'static Pack>,")?;
    writeln!(out, "    pub pack_pull_rates: &'static [CardPackPullRate],")?;
    writeln!(out, "}}\n")?;

    writeln!(out, "pub struct CardPackPullRate {{")?;
    writeln!(out, "    pub pack: &'static Pack,")?;
    writeln!(out, "    pub percent: f64,")?;
    writeln!(out, "    pub prob: Prob,")?;
    writeln!(out, "}}\n")?;

    writeln!(out, "pub static CARD_PULL_RATES: &[CardPullRates] = &[")?;
    for card in CardVersion::ALL {
        let num_packs = card.packs().len();
        let mut best_pack = 0usize;
        let mut max_pull_rate = Prob::ZERO;
        pull_rates.clear();
        pull_rates.resize(num_packs, Prob::ZERO);
        for pack in card.packs() {
            let pull_rate = card_pull_rate(pack, card);
            pull_rates[pack.id() % num_packs] = pull_rate;
            if pull_rate > max_pull_rate {
                best_pack = pack.id();
                max_pull_rate = pull_rate;
            }
        }

        writeln!(out, "    CardPullRates {{")?;
        writeln!(
            out,
            "        max_pull_rate_pct: f64::from_bits({}),",
            (max_pull_rate.as_f64() * 100.0).to_bits()
        )?;
        writeln!(
            out,
            "        max_pull_rate: Prob::new({}, {}),",
            max_pull_rate.numerator(),
            max_pull_rate.denominator()
        )?;
        if max_pull_rate > Prob::ZERO && !card.set().is_promo() {
            writeln!(
                out,
                "        best_pack: Some(Pack::from_id({best_pack}).unwrap()),"
            )?;
        } else {
            writeln!(out, "        best_pack: None,")?;
        }
        writeln!(out, "        pack_pull_rates: &[")?;
        for (idx, pull_rate) in pull_rates.iter().enumerate() {
            writeln!(out, "            CardPackPullRate {{")?;
            let mut pack_id = 0usize;
            for pack in card.packs() {
                if pack.id() % num_packs == idx {
                    pack_id = pack.id();
                    break;
                }
            }
            writeln!(
                out,
                "                pack: Pack::from_id({pack_id}).unwrap(),"
            )?;
            writeln!(
                out,
                "                percent: f64::from_bits({}),",
                (pull_rate.as_f64() * 100.0).to_bits()
            )?;
            writeln!(
                out,
                "                prob: Prob::new({}, {}),",
                pull_rate.numerator(),
                pull_rate.denominator()
            )?;
            writeln!(out, "            }},")?;
        }
        writeln!(out, "        ],")?;
        writeln!(out, "    }},")?;
    }
    writeln!(out, "];")?;

    Ok(())
}

fn card_pull_rate(pack: &Pack, card: &CardVersion) -> Prob {
    let card_id = card.id();
    let mut total = Prob::ZERO;
    for variant in pack.variants() {
        let mut not_prob = Prob::ONE;
        for slot in variant.slots() {
            for cvpr in slot.card_versions() {
                if cvpr.card_version().id() == card_id {
                    not_prob *= Prob::ONE.saturating_sub(&cvpr.pull_rate());
                    break;
                }
            }
        }
        total =
            (total + (Prob::ONE.saturating_sub(&not_prob) * variant.pull_rate())).min(Prob::ONE);
    }
    total
}
