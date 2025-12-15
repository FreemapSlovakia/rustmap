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

    let sql = "SELECT name, geometry
        FROM osm_places
        WHERE type IN ('locality', 'city_block', 'plot') AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
        ORDER BY z_order DESC, population DESC, osm_id";

    let text_options = TextOptions {
        flo: FontAndLayoutOptions {
            size: 11.0,
            ..FontAndLayoutOptions::default()
        },
        halo_opacity: 0.2,
        color: colors::LOCALITY_LABEL,
        ..TextOptions::default()
    };

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(1024.0)).as_params())
        .expect("db data");

    for row in rows {
        draw_text(
            context,
            Some(collision),
            &geometry_point(&row).project_to_tile(&ctx.tile_projector),
            row.get("name"),
            &text_options,
        );
    }
}
