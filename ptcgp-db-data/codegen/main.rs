use anyhow::{Context, Result};

mod generate;
mod load;
mod marshal;
mod models;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let raw_data = load::RawData::load().context("failed to load JSON data")?;
    let data = marshal::marshal(raw_data).context("failed to marshal data")?;
    let code = generate::generate(data);
    let file: syn::File = syn::parse2(code).context("failed to parse generated code")?;
    let output = prettyplease::unparse(&file);

    let out_dir = std::env::var_os("OUT_DIR")
        .ok_or_else(|| anyhow::anyhow!("OUT_DIR env variable not set"))?;
    let out_dir: &std::path::Path = out_dir.as_ref();
    let out_file = out_dir.join("generated.rs");
    std::fs::write(out_file, output).context("failed to write generated code")?;

    Ok(())
}

fn make_slug(subtitle: &str) -> String {
    let nfkd = icu_normalizer::DecomposingNormalizerBorrowed::new_nfkd();
    let mut slug = String::with_capacity(subtitle.len());
    for c in nfkd.normalize(&subtitle.to_lowercase()).chars() {
        if c.is_ascii_alphanumeric() || c == '-' {
            slug.push(c);
        } else if c.is_whitespace() && !slug.ends_with('_') {
            slug.push('_')
        }
    }
    slug.trim_matches('_').to_string()
}
