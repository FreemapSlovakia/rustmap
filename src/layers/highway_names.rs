use crate::{
    bbox::BBox,
    collision::Collision,
    colors::{self},
    ctx::Ctx,
    draw::text_on_line::{TextOnLineOptions, text_on_line},
    projectable::Projectable,
};
use geo::Coord;
use postgis::ewkb::LineString;
use postgres::Client;

pub fn highway_names(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        ..
    } = ctx;

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

    let buffer = ctx.meters_per_pixel() * 512.0;

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    let options = TextOnLineOptions {
        repeat_distance: Some(200.0),
        spacing: 200.0,
        color: colors::TRACK,
        ..TextOnLineOptions::default()
    };

    for row in rows {
        let geom: LineString = row.get("geometry");

        let name: &str = row.get("name");

        let projected: Vec<Coord> = geom.points.iter().map(|p| p.project(ctx)).collect();

        text_on_line(ctx, projected, name, Some(collision), &options);
    }
}
