use postgis::ewkb::{LineString, Point};
use postgres::Client;

use crate::{
    bbox::BBox,
    collision::Collision,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{
        draw::Projectable,
        text_on_line::{Align, TextOnLineOptions, Upright, text_on_line},
    },
};

pub fn highway_names(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        context,
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
          SELECT name, ST_LineMerge(ST_Collect(geometry)) AS geom, type, z_order
          FROM osm_roads
          WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND name <> ''
          GROUP BY z_order, name, type
        )
        SELECT name, (ST_Dump(ST_CollectionExtract(geom, 2))).geom AS geometry, type
        FROM merged
        ORDER BY z_order DESC";

    let buffer = ctx.meters_per_pixel() * 256.0;

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    for row in rows {
        let geom: LineString = row.get("geometry");

        let name: &str = row.get("name");

        text_on_line(
            ctx,
            geom.points.iter(),
            name,
            &TextOnLineOptions {
                repeat_distance: Some(50.0),
                spacing: 50.0,
                color: colors::TRACK,
                ..TextOnLineOptions::default()
            },
        );
    }
}
