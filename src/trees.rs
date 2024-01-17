use crate::{ctx::Ctx, draw::Projectable};
use postgis::ewkb::Point;
use postgres::Client;
use rsvg::Loader;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        context,
        ..
    } = ctx;

    let zoom = ctx.zoom;

    // TODO lazy

    let handle = Loader::new().read_path("images/tree2.svg").unwrap();

    let renderer = rsvg::CairoRenderer::new(&handle);

    let dim = renderer.intrinsic_size_in_pixels().unwrap();

    for row in &client.query(
      "SELECT type, geometry
      FROM osm_features
      WHERE
        geometry && make_buffered_envelope($1, $2, $3, $4, $5, 32) AND (
          type = 'tree' AND (NOT (tags ? 'protected') OR tags->'protected' = 'no') AND (NOT (tags ? 'denotation') OR tags->'denotation' <> 'natural_monument')
          OR type = 'shrub'
        )
        ORDER BY type, st_x(geometry)",
      &[min_x, min_y, max_x, max_y, &(zoom as i32)]
  ).unwrap() {
      let geometry: Point = row.get("geometry");

      let typ: &str = row.get("type");

      let point = geometry.project(ctx);

      let z = (2.0 + 2f64.powf(zoom as f64 - 15.0)) * (if typ == "shrub" {0.115} else {0.23});

      renderer
            .render_document(&context, &cairo::Rectangle::new(
                point.0 - z * dim.0 / 2.0,
                point.1 - z * dim.1 / 2.0,
                z * dim.0,
                z * dim.1)
            ).unwrap();
  }
}
