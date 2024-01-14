use postgis::ewkb::Geometry;
use postgres::Client;

use crate::{ctx::Ctx, draw::draw_mpoly};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    for row in &client.query(
        "SELECT type, geometry FROM osm_buildings WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        &[min_x, min_y, max_x, max_y]
    ).unwrap() {
        let geom: Geometry = row.get("geometry");

        context.set_source_rgb(0.5, 0.5, 0.5);

        draw_mpoly(ctx, &geom);

        context.fill().unwrap();
    }
}
