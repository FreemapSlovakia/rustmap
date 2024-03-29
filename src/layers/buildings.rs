use postgis::ewkb::Geometry;
use postgres::Client;

use crate::{bbox::BBox, ctx::Ctx, draw::draw::draw_geometry};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: BBox { min_x, min_y, max_x, max_y },
        ..
    } = ctx;

    for row in &client.query(
        "SELECT type, geometry FROM osm_buildings WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        &[min_x, min_y, max_x, max_y]
    ).unwrap() {
        let geom: Geometry = row.get("geometry");

        context.set_source_rgb(0.5, 0.5, 0.5);

        draw_geometry(ctx, &geom);

        context.fill().unwrap();
    }
}
