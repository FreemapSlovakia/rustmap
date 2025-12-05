use crate::{
    bbox::BBox,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        smooth_line::draw_smooth_bezier_spline,
        text_on_line::{TextOnLineOptions, Upright, text_on_line},
    },
};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, country: &str) {
    let Ctx {
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        context,
        zoom,
        ..
    } = ctx;

    if *zoom < 12 {
        return;
    }

    let simplify_factor: f64 = match zoom {
        ..=12 => 2000.0,
        13 => 1000.0,
        14 => 200.0,
        15 => 50.0,
        _ => 0.0,
    };

    context.save().expect("context saved");

    // TODO measure performance impact of simplification, if it makes something faster

    let sql = format!(
        "SELECT ST_SimplifyVW((ST_Dump(ST_LineMerge(ST_Collect(ST_ClipByBox2D(wkb_geometry,
            ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), 100)))))).geom, $5) AS geom,
            height
            FROM contour_{}_split
            WHERE wkb_geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
            GROUP BY height",
        country
    );

    let rows = client
        .query(&sql, &[min_x, min_y, max_x, max_y, &simplify_factor])
        .unwrap_or_default();

    for row in rows {
        let height: f64 = row.get("height");

        let width = match zoom {
            12 if height % 50.0 == 0.0 => 0.2,
            13..=14 if height % 100.0 == 0.0 => 0.4,
            13..=14 if height % 20.0 == 0.0 => 0.2,
            15.. if height % 100.0 == 0.0 => 0.6,
            15.. if height % 10.0 == 0.0 => 0.3,
            15.. if height % 50.0 == 0.0 && height % 100.0 != 0.0 => 0.0,
            _ => 0.0,
        };

        let labels = match zoom {
            12 if height % 50.0 == 0.0 => false,
            13..=14 if height % 100.0 == 0.0 => true,
            13..=14 if height % 20.0 == 0.0 => false,
            15.. if height % 50.0 == 0.0 => true,
            _ => false,
        };

        let geom: LineString = row.get("geom");

        if width > 0.0 {
            context.set_dash(&[], 0.0);

            context.set_line_width(width);

            context.set_source_color(colors::CONTOUR);

            draw_smooth_bezier_spline(ctx, geom.points.iter(), 1.0);

            context.stroke().unwrap();
        }

        if labels {
            text_on_line(
                ctx,
                geom.points.iter(),
                &format!("{}", height),
                None,
                &TextOnLineOptions {
                    flo: FontAndLayoutOptions {
                        ..FontAndLayoutOptions::default()
                    },
                    upright: Upright::Right,
                    color: colors::CONTOUR,
                    ..TextOnLineOptions::default()
                },
            );
        }
    }

    context.restore().expect("context restored");
}
