use crate::{
    collision::Collision,
    colors::{self},
    ctx::Ctx,
    draw::text_on_line::{TextOnLineOptions, draw_text_on_line},
    projectable::{TileProjectable, geometry_line_string},
};

use postgres::Client;

pub fn highway_names(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let sql = "
        WITH merged AS (
          SELECT name, ST_LineMerge(ST_Collect(geometry)) AS geom, type, z_order, MIN(osm_id) AS osm_id
          FROM osm_roads
          WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND name <> ''
          GROUP BY z_order, name, type
        )
        SELECT name, (ST_Dump(ST_CollectionExtract(geom, 2))).geom AS geometry, type
        FROM merged
        ORDER BY z_order DESC, osm_id";

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(512.0)).as_params())
        .expect("db data");

    let options = TextOnLineOptions {
        spacing: Some(200.0),
        color: colors::TRACK,
        ..TextOnLineOptions::default()
    };

    for row in rows {
        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        let name: &str = row.get("name");

        draw_text_on_line(ctx.context, &geom, name, Some(collision), &options);
    }
}
