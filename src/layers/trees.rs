use crate::{
    ctx::Ctx,
    projectable::{TileProjectable, geometry_point},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let mut svg_cache = ctx.svg_cache.borrow_mut();

    let zoom = ctx.zoom;

    let sql = "SELECT type, geometry
      FROM osm_features
      WHERE
        geometry && make_buffered_envelope($1, $2, $3, $4, $5, 32) AND (
          type = 'tree' AND (NOT (tags ? 'protected') OR tags->'protected' = 'no') AND (NOT (tags ? 'denotation') OR tags->'denotation' <> 'natural_monument')
          OR type = 'shrub'
        )
        ORDER BY type, st_x(geometry)";

    let mut params = ctx.bbox_query_params(None);
    params.push(zoom as i32);

    let rows = client.query(sql, &params.as_params()).expect("db data");

    for row in rows {
        let typ: &str = row.get("type");

        let point = geometry_point(&row).project_to_tile(&ctx.tile_projector);

        let scale =
            (2.0 + 2f64.powf(zoom as f64 - 15.0)) * (if typ == "shrub" { 0.1 } else { 0.2 });

        let surface = svg_cache.get("images/tree2.svg");

        let rect = surface.extents().unwrap();

        context.save().unwrap();

        context.translate(
            point.x() - scale * rect.width() / 2.0,
            point.y() - scale * rect.height() / 2.0,
        );

        context.scale(scale, scale);

        context.set_source_surface(surface, 0.0, 0.0).unwrap();

        context.paint().unwrap();

        context.restore().unwrap();
    }
}
