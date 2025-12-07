use cairo::Context;
use pangocairo::{
    functions::create_layout,
    pango::{
        Alignment, AttrInt, AttrList, FontDescription, Layout, SCALE, Style, Weight, WrapMode,
    },
};

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

impl Default for FontAndLayoutOptions {
    fn default() -> Self {
        FontAndLayoutOptions {
            letter_spacing: 0.0,
            max_width: 100.0,
            narrow: false,
            size: 12.0,
            style: Style::Normal,
            uppercase: false,
            weight: Weight::Normal,
        }
    }
}

pub fn create_pango_layout(
    context: &Context,
    text: &str,
    options: &FontAndLayoutOptions,
) -> Layout {
    let FontAndLayoutOptions {
        letter_spacing,
        max_width,
        narrow,
        size,
        style,
        uppercase,
        weight,
    } = options;

    let layout = create_layout(context);

    let mut font_description = FontDescription::new();

    font_description.set_family(if *narrow {
        "PT Sans Narrow,Fira Sans Extra Condensed,Noto Sans"
    } else {
        "PT Sans,Fira Sans Condensed,Noto Sans"
    });

    font_description.set_weight(*weight);

    font_description.set_size((SCALE as f64 * size * 0.75) as i32);

    font_description.set_style(*style);

    // font_description.set_variant(Variant::SmallCaps);

    layout.set_font_description(Some(&font_description));

    layout.set_wrap(WrapMode::Word);
    layout.set_alignment(Alignment::Center);
    layout.set_line_spacing(0.4);
    layout.set_width((max_width * SCALE as f64) as i32);

    let text = if *uppercase {
        &text.to_uppercase()
    } else {
        text
    };

    layout.set_text(text);

    // layout.set_markup(r#"<span font_features="liga=1">fi</span>"#);

    if *letter_spacing != 1.0 {
        let attr_list = AttrList::new();

        attr_list.insert(AttrInt::new_letter_spacing(
            (SCALE as f64 * *letter_spacing) as i32,
        ));

        layout.set_attributes(Some(&attr_list));
    }

    layout
}
