use std::sync::LazyLock;

use crate::{
    collision::{self, Collision},
    colors,
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        offset_line::offset_line_string,
        text_on_line::{TextOnLineOptions, draw_text_on_line},
    },
    projectable::{TileProjectable, geometry_line_string},
    re_replacer::{Replacement, replace},
};
use geo::ChaikinSmoothing;
use pangocairo::pango::Style;
use postgres::{Client, Row};
use regex::Regex;

static REPLACEMENTS: LazyLock<Vec<Replacement>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"^Dolink?a\b *").unwrap(), "Dol. "),
        (Regex::new(r"^dolink?a\b *").unwrap(), "dol. "),
        (Regex::new(r" *\b[Dd]olink?a$").unwrap(), " dol."),
    ]
});

pub fn render(ctx: &Ctx, client: &mut Client) {
    let zoom_coef = 2.5f64.powf(ctx.zoom as f64 - 12.0);

    let opacity = 0.5 - (ctx.zoom as f64 - 13.0) / 10.0;
    let cs = 15.0 + zoom_coef;
    let size = 10.0 + zoom_coef;
    let off = 6.0 + 3.0 * zoom_coef;

    let context = ctx.context;

    let collision = &mut Collision::new(None);

    let mut render_rows = |rows: Vec<Row>| {
        for row in rows {
            let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

            let of: f64 = row.get("offset_factor");

            // TODO offset should be negative depending of upright placement
            let geom = offset_line_string(&geom, of * off);

            let geom = geom.chaikin_smoothing(3);

            let mut options = TextOnLineOptions {
                flo: FontAndLayoutOptions {
                    style: Style::Italic,
                    letter_spacing: cs,
                    size,
                    ..Default::default()
                },
                color: colors::TRAM,
                halo_opacity: 0.9,
                spacing: Some(200.0),
                ..Default::default()
            };

            while options.flo.letter_spacing >= 1.0 {
                let drawn = draw_text_on_line(
                    context,
                    &geom,
                    &replace(row.get("name"), &REPLACEMENTS),
                    Some(collision),
                    &options,
                );

                if drawn {
                    break;
                }

                options.flo.letter_spacing *= 0.9;
            }
        }
    };

    context.push_group();

    let sql = "
        SELECT
            geometry, name, LEAST(1.2, ST_Length(geometry) / 5000) AS offset_factor
        FROM
            osm_feature_lines
        WHERE
            type = 'valley' AND name <> '' AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
        ORDER BY
            ST_Length(geometry) DESC";

    render_rows(
        client
            .query(sql, &ctx.bbox_query_params(Some(512.0)).as_params())
            .expect("db data"),
    );

    let sql = "
        SELECT
            geometry, name, 0::double precision AS offset_factor
        FROM
            osm_feature_lines
        WHERE
            type = 'ridge' AND name <> '' AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
        ORDER BY
            ST_Length(geometry) DESC";

    render_rows(
        client
            .query(sql, &ctx.bbox_query_params(Some(512.0)).as_params())
            .expect("db data"),
    );

    context.pop_group_to_source().unwrap();

    context.paint_with_alpha(opacity).unwrap();
}
