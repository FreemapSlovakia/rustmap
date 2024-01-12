use crate::{colors::{self, ContextExt}, ctx::Ctx};
use postgis::ewkb::Geometry;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
  //   let Ctx {
  //       bbox: (min_x, min_y, max_x, max_y),
  //       context,
  //       zoom,
  //       ..
  //   } = ctx;

  //   for row in &client.query(
  //       &format!("SELECT type, {} FROM osm_waterareas WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
  //     match zoom {
  //       12 => "ST_Segmentize(ST_Simplify(geometry, 24), 200) AS geometry",
  //       13 => "ST_Segmentize(ST_Simplify(geometry, 12), 200) AS geometry",
  //       14 => "ST_Segmentize(ST_Simplify(geometry, 6), 200) AS geometry",
  //       _ => "geometry"
  //     }),
  //       &[min_x, min_y, max_x, max_y]
  //   ).unwrap() {
  //       let geom: Geometry = row.get("geometry");

  //       context.set_source_color(*colors::WATER);

  //       // TODO
  // }
}
