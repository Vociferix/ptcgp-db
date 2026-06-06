use dioxus::prelude::*;

enum Segment {
    Text(String),
    Symbol(&'static str),
}

fn parse_segments(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(pos) = rest.find('[') {
            let after = &rest[pos + 1..];
            if after.len() >= 2
                && after.as_bytes()[0].is_ascii_uppercase()
                && after.as_bytes()[1] == b']'
            {
                let code = after.as_bytes()[0] as char;
                if let Some(elem) = ptcgp_db_data::Element::ALL
                    .iter()
                    .find(|e| e.code() == Some(code))
                {
                    if pos > 0 {
                        segments.push(Segment::Text(rest[..pos].to_string()));
                    }
                    segments.push(Segment::Symbol(elem.symbol()));
                    rest = &after[2..];
                    continue;
                }
            }
            segments.push(Segment::Text(rest[..pos + 1].to_string()));
            rest = &rest[pos + 1..];
        } else {
            segments.push(Segment::Text(rest.to_string()));
            break;
        }
    }
    segments
}

/// Renders effect text with element placeholders (`[R]`, `[G]`, etc.) replaced by inline
/// element symbol images.
#[component]
pub fn EffectText(text: String, #[props(default)] class: String) -> Element {
    let segments = parse_segments(&text);
    rsx! {
        span { class: "inline leading-relaxed {class}",
            for seg in segments {
                match seg {
                    Segment::Text(t) => rsx! { "{t}" },
                    Segment::Symbol(asset) => rsx! {
                        img {
                            src: "{asset}",
                            alt: "",
                            class: "inline h-4 w-4 mx-0.5 align-middle object-contain",
                        }
                    },
                }
            }
        }
    }
}
