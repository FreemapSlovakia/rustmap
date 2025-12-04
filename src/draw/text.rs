use std::f64::consts::PI;

use crate::{
    bbox::BBox,
    collision::Collision,
    colors::{self, Color, ContextExt},
    point::Point,
};
use cairo::Context;
use pangocairo::{
    functions::{create_layout, glyph_string_path, layout_path},
    pango::{
        Alignment, AttrInt, AttrList, FontDescription, GlyphString, SCALE, Style, Weight, WrapMode,
        ffi::{PangoGlyphString, pango_glyph_string_new},
    },
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
    pub style: Style,
    pub uppercase: bool,
    pub weight: Weight,
}

pub static DEFAULT_PLACEMENTS: &[f64] = &[0.0, 3.0, -3.0, 6.0, -6.0, 9.0, -9.0];

impl Default for TextOptions<'_> {
    fn default() -> Self {
        TextOptions {
            alpha: 1.0,
            color: colors::BLACK,
            halo_color: colors::WHITE,
            halo_opacity: 0.75,
            halo_width: 1.5,
            letter_spacing: 0.0,
            max_width: 100.0,
            narrow: false,
            placements: &[0.0],
            size: 12.0,
            style: Style::Normal,
            uppercase: false,
            weight: Weight::Normal,
        }
    }
}

pub fn draw_text(
    context: &Context,
    collision: &mut Collision<f64>,
    point: Point,
    original_text: &str,
    options: &TextOptions,
) {
    if original_text.is_empty() {
        return;
    }

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

    font_description.set_size((SCALE as f64 * size * 0.75) as i32);

    font_description.set_style(*style);

    layout.set_font_description(Some(&font_description));

    let uppercase_text;

    let text = if *uppercase {
        uppercase_text = original_text.to_uppercase();
        &uppercase_text
    } else {
        original_text
    };

    // let text = "الله";

    layout.set_wrap(WrapMode::Word);
    layout.set_alignment(Alignment::Center);
    layout.set_line_spacing(0.4);
    layout.set_width((max_width * SCALE as f64) as i32);

    layout.set_text(text);
    // layout.set_markup(r#"<span font_features="liga=1">fi</span>"#);

    // let letter_spacing = &16.0;

    if *letter_spacing != 1.0 {
        let attr_list = AttrList::new();

        attr_list.insert(AttrInt::new_letter_spacing(
            (SCALE as f64 * *letter_spacing) as i32,
        ));

        layout.set_attributes(Some(&attr_list));
    }

    let x = point.x - max_width / 2.0;

    let mut my: Option<f64> = None;

    let (ext, _) = layout.extents();

    // let layout_x = ext.x() as f64 / SCALE as f64;

    // let layout_y = ext.y() as f64 / SCALE as f64;

    // let layout_width = ext.width() as f64 / SCALE as f64;

    let layout_height = ext.height() as f64 / SCALE as f64;

    'outer: for dy in *placements {
        let y = *dy + point.y - layout_height / 2.0;

        // let ci = BBox::new(
        //     x - halo_width + layout_x,
        //     y - halo_width + layout_y,
        //     x + 2.0 * halo_width + layout_x + layout_width,
        //     y + 2.0 * halo_width + layout_y + layout_height,
        // );

        // if collision.collides(ci) {
        //     continue;
        // }

        // collision.add(ci);

        // context.rectangle(ci.min_x, ci.min_y, ci.get_width(), ci.get_height());
        // context.set_line_width(1.0);
        // context.set_source_rgb(0.0, 0.0, 1.0);
        // context.stroke().unwrap();

        let mut items = Vec::new();

        let mut li = layout.iter();

        // let t = layout.text();

        // println!("==================");

        // let mut n = 0;

        // loop {
        //     let (ext, _) = li.cluster_extents();

        //     let char_x = ext.x() as f64 / SCALE as f64;

        //     let char_y = ext.y() as f64 / SCALE as f64;

        //     let char_width = ext.width() as f64 / SCALE as f64;

        //     let char_height = ext.height() as f64 / SCALE as f64;

        //     let ci = BBox::new(
        //         x - halo_width + char_x,
        //         y - halo_width + char_y,
        //         x + 2.0 * halo_width + char_x + char_width,
        //         y + 2.0 * halo_width + char_y + char_height,
        //     );

        //     context.rectangle(ci.min_x - 0.5, ci.min_y, ci.get_width(), ci.get_height());
        //     context.set_line_width(1.0);
        //     if n % 2 == 0 {
        //         context.set_source_rgb(0.0, 1.0, 0.0);
        //     } else {
        //         context.set_source_rgb(0.0, 0.0, 1.0);
        //     }

        //     n += 1;

        //     context.stroke().unwrap();

        //     let i = li.index() as usize;

        //     let has_next = li.next_cluster();

        //     let ni = li.index() as usize;

        //     let t = t.get(i..ni).unwrap(); // .chars().next().unwrap();
        //     // println!("II {} {}", i, t);

        //     if !has_next {
        //         // println!("XX {}", ni);

        //         break;
        //     }
        // }

        loop {
            let (ext, _) = li.line_extents();

            let line_x = ext.x() as f64 / SCALE as f64;

            let line_y = ext.y() as f64 / SCALE as f64;

            let line_width = ext.width() as f64 / SCALE as f64;

            let line_height = ext.height() as f64 / SCALE as f64;

            let ci = BBox::new(
                x - halo_width + line_x,
                y - halo_width + line_y,
                x + 2.0 * halo_width + line_x + line_width,
                y + 2.0 * halo_width + line_y + line_height,
            );

            if collision.collides(ci) {
                continue 'outer;
            }

            items.push(ci);

            // context.rectangle(ci.min_x, ci.min_y, ci.get_width(), ci.get_height());
            // context.set_line_width(1.0);
            // context.set_source_rgb(1.0, 0.0, 0.0);
            // context.stroke().unwrap();

            if !li.next_line() {
                break;
            }
        }

        for item in items {
            collision.add(item);
        }

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
}
