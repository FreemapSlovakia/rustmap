use crate::{
    bbox::BBox, colors::{self, ContextExt}, ctx::Ctx, draw::draw::draw_geometry
};
use postgis::ewkb::Geometry;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        bbox: BBox { min_x, min_y, max_x, max_y },
        context,
        ..
    } = ctx;

    for row in &client.query(
        "SELECT type, geometry FROM osm_waterareas WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        &[min_x, min_y, max_x, max_y]
    ).unwrap() {
        let geom: Geometry = row.get("geometry");

        context.set_source_color(colors::WATER);

        draw_geometry(ctx, &geom);

        context.fill().unwrap();
  }
}
