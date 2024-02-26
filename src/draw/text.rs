use crate::{
    collision::Collision,
    colors::{self, Color, ContextExt},
    point::Point,
};
use cairo::Context;
use pango::AttrInt;
use pangocairo::{
    functions::{create_layout, layout_path},
    pango::{AttrList, FontDescription},
};

pub struct TextOptions<'a> {
    pub alpha: f64,
    pub color: Color,
    pub halo_color: Color,
    pub halo_opacity: f64,
    pub halo_width: f64,
    pub letter_spacing: f64,
    pub max_width: f64,
    pub narrow: bool,
    pub placements: &'a [f64],
    pub size: f64,
    pub style: pango::Style,
    pub uppercase: bool,
    pub weight: pango::Weight,
}

pub static DEFAULT_PLACEMENTS: &[f64] = &[0.0, 3.0, -3.0, 6.0, -6.0, 9.0, -9.0];

impl Default for TextOptions<'_> {
    fn default() -> Self {
        TextOptions {
            alpha: 1.0,
            color: *colors::BLACK,
            halo_color: *colors::WHITE,
            halo_opacity: 0.75,
            halo_width: 1.5,
            letter_spacing: 0.0,
            max_width: 100.0,
            narrow: false,
            placements: &[0.0],
            size: 12.0,
            style: pango::Style::Normal,
            uppercase: false,
            weight: pango::Weight::Normal,
        }
    }
}

pub fn draw_text(
    context: &Context,
    collision: &mut Collision<f64>,
    p: Point,
    original_text: &str,
    options: &TextOptions,
) {
    let TextOptions {
        alpha,
        color,
        halo_color,
        halo_opacity,
        halo_width,
        letter_spacing,
        max_width,
        narrow,
        placements,
        size,
        style,
        uppercase,
        weight,
    } = options;

    let layout = create_layout(context);

    let max_width = max_width - 2.0 * halo_width;

    let mut font_description = FontDescription::new();
    font_description.set_family(if *narrow {
        "PT Sans Narrow,Fira Sans Extra Condensed,Noto Sans"
    } else {
        "PT Sans,Fira Sans Condensed,Noto Sans"
    });
    font_description.set_weight(*weight);
    font_description.set_size((pango::SCALE as f64 * size * 0.75) as i32);
    font_description.set_style(*style);

    layout.set_font_description(Some(&font_description));

    let uppercase_text;

    let text = if *uppercase {
        uppercase_text = original_text.to_uppercase();
        &uppercase_text
    } else {
        original_text
    };

    layout.set_wrap(pango::WrapMode::Word);
    layout.set_alignment(pango::Alignment::Center);
    layout.set_line_spacing(0.4);
    layout.set_width((max_width * pango::SCALE as f64) as i32);

    layout.set_text(text);

    let attr_list = AttrList::new();

    attr_list.insert(AttrInt::new_letter_spacing((pango::SCALE as f64 * *letter_spacing) as i32));

    layout.set_attributes(Some(&attr_list));

    let size = layout.size();

    let size = (
        size.0 as f64 / pango::SCALE as f64,
        size.1 as f64 / pango::SCALE as f64,
    );

    let x = p.x - max_width / 2.0;

    let mut my: Option<f64> = None;

    let ext = layout.pixel_extents();

    for dy in *placements {
        let y = *dy + p.y - size.1 as f64 / 2.0;

        let ci = (
            (
                x - halo_width + ext.0.x() as f64,
                x + 2.0 * halo_width + (ext.0.x() + ext.0.width()) as f64,
            ),
            (
                y - halo_width + ext.0.y() as f64,
                y + 2.0 * halo_width + (ext.0.y() + ext.0.height()) as f64,
            ),
        );

        if collision.collides(ci) {
            continue;
        }

        collision.add(ci);

        my = Some(y);

        break;
    }

    let y = match my {
        Some(y) => y,
        None => return,
    };

    context.move_to(x, y);

    layout_path(context, &layout);

    context.push_group();

    context.set_source_color_a(*halo_color, *halo_opacity);
    context.set_dash(&[], 0.0);
    context.set_line_width(halo_width * 2.0);

    context.stroke_preserve().unwrap();

    context.set_source_color(*color);

    context.fill().unwrap();

    context.pop_group_to_source().unwrap();

    context.paint_with_alpha(*alpha).unwrap();

    // context.set_source_rgb(1.0, 0.0, 0.0);
    // context.set_dash(&[], 0.0);

    // let ext = layout.pixel_extents();

    // context.rectangle(
    //     x + ext.0.x() as f64,
    //     y + ext.0.y() as f64,
    //     ext.0.width() as f64,
    //     ext.0.height() as f64,
    // );
    // context.set_line_width(1.0);
    // context.stroke().unwrap();
}
