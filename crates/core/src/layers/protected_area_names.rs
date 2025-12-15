use crate::{
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text::{TextOptions, draw_text},
        text_on_line::{Align, Distribution, Repeat, TextOnLineOptions, draw_text_on_line},
    },
    layers::national_park_names::REPLACEMENTS,
    projectable::{TileProjectable, geometry_line_string, geometry_point},
    re_replacer::replace,
};
use pangocairo::pango::Style;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let _span = tracy_client::span!("protected_area_names::render");

    let sql = "SELECT name, ST_Centroid(geometry) AS geometry
        FROM osm_protected_areas
        WHERE
            geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
                AND (type = 'nature_reserve' OR (type = 'protected_area' AND protect_class <> '2'))
        ORDER BY area DESC";

    let text_options = TextOptions {
        flo: FontAndLayoutOptions {
            style: Style::Italic,
            ..FontAndLayoutOptions::default()
        },
        halo_opacity: 0.75,
        color: colors::PROTECTED,
        ..TextOptions::default()
    };

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(1024.0)).as_params())
        .expect("db data");

    for row in rows {
        draw_text(
            ctx.context,
            Some(collision),
            &geometry_point(&row).project_to_tile(&ctx.tile_projector),
            row.get("name"),
            &text_options,
        );
    }

    let sql = "SELECT type, name, protect_class, (ST_Dump(ST_Boundary(geometry))).geom AS geometry
        FROM osm_protected_areas
        WHERE
            geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND
            (type = 'national_park' OR (type = 'protected_area' AND protect_class = '2'))
        ORDER BY area DESC";

    let text_options = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            style: Style::Italic,
            ..FontAndLayoutOptions::default()
        },
        alpha: 0.66,
        halo_opacity: 0.75,
        color: colors::PROTECTED,
        offset: -14.0,
        distribution: Distribution::Align {
            align: Align::Center,
            repeat: Repeat::Spaced(600.0),
        },
        keep_offset_side: true,
        ..TextOnLineOptions::default()
    };

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(1024.0)).as_params())
        .expect("db data");

    for row in rows {
        draw_text_on_line(
            ctx.context,
            &geometry_line_string(&row).project_to_tile(&ctx.tile_projector),
            &replace(row.get("name"), &REPLACEMENTS),
            Some(collision),
            &text_options,
        );
    }
}
