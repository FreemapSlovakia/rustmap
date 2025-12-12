use crate::colors::{self, ContextExt};
use crate::ctx::Ctx;
use crate::draw::create_pango_layout::FontAndLayoutOptions;
use crate::draw::offset_line::offset_line;
use crate::draw::text_on_line::{Align, TextOnLineOptions, text_on_line};
use crate::layers::borders;
use crate::projectable::{TileProjectable, geometry_line_string};
use postgres::Client;
use std::f64;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let rect = ctx.bbox.project_to_tile(&ctx.tile_projector);

    context.save().unwrap();
    context.rectangle(rect.min().x, rect.min().y, rect.width(), rect.height());
    context.set_source_color_a(colors::WHITE, 0.33);
    context.fill().unwrap();
    context.restore().unwrap();

    borders::render(ctx, client);

    let sr = 1.5f64.powf(ctx.zoom as f64 - 6.0);

    let options_upper = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            size: sr * 20.0,
            ..Default::default()
        },
        halo_width: 2.0,
        align: Align::Justify {
            min_spacing: Some(0.0),
        },
        concave_spacing_factor: 0.0,
        ..Default::default()
    };

    let options_lower = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            size: sr * 16.0,
            ..Default::default()
        },
        halo_width: 2.0,
        color: colors::AREA_LABEL,
        align: Align::Justify {
            min_spacing: Some(0.0),
        },
        concave_spacing_factor: 0.0,
        ..Default::default()
    };

    let sql = concat!(
        r#"SELECT name, "name:en",  geometry FROM country_names_smooth "#,
        "WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(20.0)).as_params())
        .expect("db data");

    for row in rows {
        let name: &str = row.get("name");

        let name_en: &str = row.get("name:en");

        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        // context.save().unwrap();
        // draw_line(context, &geom);
        // context.set_source_color(colors::BLACK);
        // context.set_line_width(2.0);
        // context.stroke().unwrap();
        // context.restore().unwrap();

        context.push_group();

        for (name, mut offset, options) in [
            (name, sr * -14.0, &options_upper),
            (name_en, sr * 8.0, &options_lower),
        ] {
            let mut options = *options;

            for _ in 0..4 {
                let geom = offset_line(&geom, offset);

                if text_on_line(context, &geom, name, None, &options) {
                    break;
                }

                options.flo.size *= 0.9;
                options.halo_width *= 0.9;
                offset *= 0.9;
            }
        }

        context.pop_group_to_source().unwrap();
        context.paint_with_alpha(0.66).unwrap();
    }
}
