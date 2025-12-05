use crate::{
    bbox::BBox,
    collision::Collision,
    colors::{self, Color, ContextExt},
    draw::create_pango_layout::{FontAndLayoutOptions, create_pango_layout},
    point::Point,
};
use cairo::Context;
use pangocairo::{functions::layout_path, pango::SCALE};

#[derive(Copy, Clone, Debug)]
pub struct TextOptions<'a> {
    pub alpha: f64,
    pub color: Color,
    pub halo_color: Color,
    pub halo_opacity: f64,
    pub halo_width: f64,
    pub placements: &'a [f64],
    pub flo: FontAndLayoutOptions,
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
            flo: FontAndLayoutOptions::default(),
            placements: &[0.0],
        }
    }
}

pub fn draw_text(
    context: &Context,
    collision: &mut Collision<f64>,
    point: Point,
    text: &str,
    options: &TextOptions,
) {
    if text.is_empty() {
        return;
    }

    let TextOptions {
        alpha,
        color,
        halo_color,
        halo_opacity,
        halo_width,
        placements,
        flo,
    } = options;

    // context.save().expect("context saved");

    // context.arc(point.x, point.y, 2.0, 0.0, TAU);
    // context.set_source_color(colors::BLACK);

    // context.fill().unwrap();
    // context.restore().expect("context restored");

    let layout = create_pango_layout(context, text, flo);

    let mut my: Option<f64> = None;

    let (ext, _) = layout.extents();

    let layout_x = ext.x() as f64 / SCALE as f64;

    let layout_y = ext.y() as f64 / SCALE as f64;

    let layout_width = ext.width() as f64 / SCALE as f64;

    let layout_height = ext.height() as f64 / SCALE as f64;

    let x = point.x - (layout_x + layout_width / 2.0);

    'outer: for dy in *placements {
        let y = *dy + point.y - (layout_y + layout_height / 2.0);

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

    context.save().expect("context saved");

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

    context.restore().expect("context restored");
}
