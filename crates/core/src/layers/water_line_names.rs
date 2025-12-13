use crate::{
    collision::Collision,
    colors::{self},
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text_on_line::{Align, Distribution, Repeat, TextOnLineOptions, draw_text_on_line},
    },
    projectable::{TileProjectable, geometry_line_string},
    re_replacer::{Replacement, replace},
};
use pangocairo::pango::Style;
use postgres::Client;
use regex::Regex;
use std::sync::LazyLock;

static REPLACEMENTS: LazyLock<Vec<Replacement>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"\b[Pp]otok$").unwrap(), "p."),
        (Regex::new(r"^[Pp]otok\b *").unwrap(), ""),
    ]
});

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let zoom = ctx.zoom;

    let sql = format!(
        "WITH merged AS (
            SELECT
                ST_LineMerge(ST_Collect(ST_Segmentize(ST_Simplify(geometry, 24), 200))) AS geometry,
                name, type, MIN(osm_id) AS osm_id
            FROM osm_waterways
            WHERE name <> '' {}AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
            GROUP BY name, type
        )
        SELECT name, (ST_Dump(ST_CollectionExtract(geometry, 2))).geom AS geometry
        FROM merged ORDER BY osm_id, type", // TODO order by type - river 1st
        if zoom < 14 { "AND type = 'river' " } else { "" }
    );

    let rows = client
        .query(&sql, &ctx.bbox_query_params(Some(1024.0)).as_params())
        .expect("db data");

    let options = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            style: Style::Italic,
            letter_spacing: 2.0,
            ..FontAndLayoutOptions::default()
        },
        distribution: Distribution::Align {
            align: Align::Center,
            repeat: Repeat::Spaced(300.0),
        },
        color: colors::WATER_LABEL,
        halo_color: colors::WATER_LABEL_HALO,
        ..TextOnLineOptions::default()
    };

    for row in rows {
        draw_text_on_line(
            ctx.context,
            &geometry_line_string(&row).project_to_tile(&ctx.tile_projector),
            &replace(row.get("name"), &REPLACEMENTS),
            Some(collision),
            &options,
        );
    }
}
