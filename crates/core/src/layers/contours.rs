use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        smooth_line::path_smooth_bezier_spline,
        text_on_line::{
            Align, Distribution, Repeat, TextOnLineOptions, Upright, draw_text_on_line,
        },
    },
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, country: Option<&str>) {
    let _span = tracy_client::span!("contours::render");

    let context = ctx.context;
    let zoom = ctx.zoom;

    if zoom < 12 {
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
        "SELECT ST_SimplifyVW(wkb_geometry, $6) AS geometry, height
        FROM {}
        WHERE wkb_geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)",
        &(if let Some(country) = country {
            format!("contour_{country}_split")
        } else {
            "contour_split".to_string()
        })
    );

    let mut params = ctx.bbox_query_params(Some(8.0));

    params.push(simplify_factor);

    let query_params = params.as_params();

    let rows = client.query(&sql, &query_params).unwrap_or_default();

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

        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        if width > 0.0 {
            context.set_dash(&[], 0.0);

            context.set_line_width(width);

            context.set_source_color(colors::CONTOUR);

            path_smooth_bezier_spline(context, &geom, 1.0);

            context.stroke().unwrap();
        }

        if labels {
            draw_text_on_line(
                context,
                &geom,
                &format!("{}", height),
                None,
                &TextOnLineOptions {
                    flo: FontAndLayoutOptions::default(),
                    upright: Upright::Left,
                    color: colors::CONTOUR,
                    distribution: Distribution::Align {
                        align: Align::Center,
                        repeat: Repeat::Spaced(200.0),
                    },
                    ..TextOnLineOptions::default()
                },
            );
        }
    }

    context.restore().expect("context restored");
}
