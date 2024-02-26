use crate::{
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        draw::Projectable,
        text::{draw_text, TextOptions},
    },
};
use postgis::ewkb::Point;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let sql = "SELECT name, geometry
        FROM osm_places
        WHERE type IN ('locality', 'city_block', 'plot') AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
        ORDER BY z_order DESC, population DESC, osm_id";

    let buffer = ctx.meters_per_pixel() * 1024.0;

    for row in &client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .unwrap()
    {
        draw_text(
            context,
            collision,
            row.get::<_, Point>("geometry").project(ctx),
            row.get("name"),
            &TextOptions {
                size: 11.0,
                halo_opacity: 0.2,
                color: *colors::LOCALITY_LABEL,
                ..TextOptions::default()
            },
        );
    }
}
