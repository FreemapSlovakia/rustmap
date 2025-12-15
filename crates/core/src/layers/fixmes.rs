use crate::{
    SvgCache,
    ctx::Ctx,
    projectable::{TileProjectable, geometry_line_string, geometry_point},
};
use geo::{Coord, Euclidean, Length, LineStringSegmentize};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, svg_cache: &mut SvgCache) {
    let _span = tracy_client::span!("fixmes::render");

    let context = ctx.context;

    let sql = "
        SELECT geometry
        FROM osm_fixmes
        WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)";

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(8.0)).as_params())
        .expect("db data");

    let surface = svg_cache.get("fixme.svg");

    let rect = surface.extents().unwrap();

    let hw = rect.width() / 2.0;

    let hh = rect.height() / 2.0;

    let paint = |point: &Coord| {
        context
            .set_source_surface(surface, (point.x - hw).round(), (point.y - hh).round())
            .unwrap();

        context.paint().unwrap();
    };

    for row in rows {
        paint(&geometry_point(&row).project_to_tile(&ctx.tile_projector).0);
    }

    let sql = "
        SELECT * FROM (
            SELECT geometry, fixme FROM osm_feature_lines
            UNION
            SELECT geometry, fixme FROM osm_roads
        ) foo
        WHERE
            fixme <> '' AND
            geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)";

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(8.0)).as_params())
        .expect("db data");

    for row in rows.into_iter().skip(1) {
        let line_string = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        let Some(ml) = line_string
            .line_segmentize((Euclidean.length(&line_string) as f64 / 150.0).ceil() as usize)
        else {
            continue;
        };

        for line_string in ml {
            if let Some(c) = line_string.0.first() {
                paint(c);
            }
        }
    }
}
