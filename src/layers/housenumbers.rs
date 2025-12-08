use crate::{
    bbox::BBox,
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        draw::Projectable,
        text::{TextOptions, draw_text},
    },
};
use postgis::ewkb::Point;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        context,
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        ..
    } = ctx;

    let sql = r#"
        SELECT
            COALESCE(
                NULLIF("addr:streetnumber", ''),
                NULLIF("addr:housenumber", ''),
                NULLIF("addr:conscriptionnumber", '')
            ) AS housenumber,
            ST_PointOnSurface(geometry) AS geometry
        FROM osm_housenumbers
        WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"#;

    let buffer = ctx.meters_per_pixel() * 256.0;

    let text_options = TextOptions {
        flo: FontAndLayoutOptions {
            size: 8.0,
            ..FontAndLayoutOptions::default()
        },
        halo_opacity: 0.5,
        color: colors::AREA_LABEL,
        ..TextOptions::default()
    };

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    for row in rows {
        draw_text(
            context,
            collision,
            row.get::<_, Point>("geometry").project(ctx),
            row.get("housenumber"),
            &text_options,
        );
    }
}
