pub use cosmic_text::{Style, Weight};

#[derive(Copy, Clone, Debug)]
pub struct FontAndLayoutOptions {
    pub letter_spacing: f64,
    pub max_width: f64,
    pub narrow: bool,
    pub size: f64,
    pub style: Style,
    pub uppercase: bool,
    pub weight: Weight,
}

/// Uppercase `text` for display, leaving Georgian untouched.
///
/// Georgian is a unicameral script: `str::to_uppercase` maps Mkhedruli
/// (U+10D0–U+10FF) to Mtavruli (Georgian Extended, U+1C90–U+1CBF), a titling
/// style our bundled fonts don't cover — so the result renders as tofu. Since
/// uppercasing Georgian is typographically wrong anyway, keep those letters as
/// is and uppercase everything else normally.
pub fn uppercase_label(text: &str) -> String {
    if text.chars().any(|c| ('\u{10D0}'..='\u{10FF}').contains(&c)) {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            if ('\u{10D0}'..='\u{10FF}').contains(&c) {
                out.push(c);
            } else {
                out.extend(c.to_uppercase());
            }
        }
        out
    } else {
        text.to_uppercase()
    }
}

impl Default for FontAndLayoutOptions {
    fn default() -> Self {
        Self {
            letter_spacing: 0.0,
            max_width: 100.0,
            narrow: false,
            size: 12.0,
            style: Style::Normal,
            uppercase: false,
            weight: Weight::NORMAL,
        }
    }
}
