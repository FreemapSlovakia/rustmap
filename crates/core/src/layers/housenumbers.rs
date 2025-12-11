use crate::{
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text::{TextOptions, draw_text},
    },
    projectable::{TileProjectable, geometry_point},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let context = ctx.context;

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
        .query(sql, &ctx.bbox_query_params(Some(128.0)).as_params())
        .expect("db data");

    for row in rows {
        draw_text(
            context,
            collision,
            &geometry_point(&row).project_to_tile(&ctx.tile_projector),
            row.get("housenumber"),
            &text_options,
        );
    }
}
