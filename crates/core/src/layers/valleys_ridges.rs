use std::sync::LazyLock;

use crate::{
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text_on_line::{Align, Distribution, Repeat, TextOnLineOptions, draw_text_on_line},
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
    let letter_spacing = 15.0 + zoom_coef;
    let size = 10.0 + zoom_coef;
    let off = 6.0 + 1.5 * zoom_coef;

    let context = ctx.context;

    let collision = &mut Collision::new(None);

    let mut render_rows = |rows: Vec<Row>| {
        for row in rows {
            let name = replace(row.get("name"), &REPLACEMENTS);

            let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

            let offset_factor: f64 = row.get("offset_factor");

            let mut options = TextOnLineOptions {
                flo: FontAndLayoutOptions {
                    style: Style::Italic,
                    letter_spacing,
                    size,
                    ..Default::default()
                },
                color: colors::TRAM,
                halo_opacity: 0.9,
                distribution: Distribution::Align {
                    align: Align::Center,
                    repeat: Repeat::Spaced(200.0),
                },
                offset: size / 2.0 + offset_factor * off,
                ..Default::default()
            };

            let geom = geom.chaikin_smoothing(3);

            while options.flo.letter_spacing >= 0.0 {
                let drawn = draw_text_on_line(context, &geom, &name, Some(collision), &options);

                if drawn {
                    break;
                }

                options.flo.letter_spacing = (options.flo.letter_spacing + 1.0) * 0.9 - 1.0;
            }

            // TODO
            // {z > 13 && <Placement characterSpacing={0} size={size * 0.75} />}
            // {z > 14 && <Placement characterSpacing={0} size={size * 0.5} />}
        }
    };

    context.push_group();

    let sql = format!("
        SELECT
            geometry, name, LEAST(1.2, ST_Length(geometry) / 5000) AS offset_factor
        FROM
            osm_feature_lines
        WHERE
            type = 'valley' AND name <> '' AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
        ORDER BY
            ST_Length(geometry) {}", if ctx.zoom > 13 {"ASC"} else {"DESC"});

    render_rows(
        client
            .query(&sql, &ctx.bbox_query_params(Some(512.0)).as_params())
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
