use std::sync::LazyLock;

use crate::{
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text::{TextOptions, draw_text},
    },
    projectable::{TileProjectable, geometry_point},
    re_replacer::{Replacement, replace},
};
use pangocairo::pango::Style;
use postgres::Client;
use regex::Regex;

static REPLACEMENTS: LazyLock<Vec<Replacement>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"\b[Oo]chranné [Pp]ásmo\b").unwrap(), "OP"),
        (Regex::new(r"\b[Nn]árodn(ého|ý) [Pp]arku?\b").unwrap(), "NP"),
    ]
});

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let context = ctx.context;

    let sql = "
        SELECT type, name, protect_class, ST_PointOnSurface(geometry) AS geometry
        FROM osm_protected_areas
        WHERE
            geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND
            (type = 'national_park' OR (type = 'protected_area' AND protect_class = '2'))
        ORDER BY name LIKE ('Ochranné pásmo %'), area DESC";

    let text_options = TextOptions {
        flo: FontAndLayoutOptions {
            style: Style::Italic,
            size: 9.0 + 2f64.powf(ctx.zoom as f64 - 7.0),
            ..FontAndLayoutOptions::default()
        },
        color: colors::PROTECTED,
        ..TextOptions::default()
    };

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(512.0)).as_params())
        .expect("db data");

    for row in rows {
        draw_text(
            context,
            collision,
            &geometry_point(&row).project_to_tile(&ctx.tile_projector),
            &replace(row.get("name"), &REPLACEMENTS),
            &text_options,
        );
    }
}
