use postgis::ewkb::{LineString, Point};
use postgres::Client;

use crate::{
    bbox::BBox,
    collision::Collision,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{
        draw::Projectable,
        text_on_line::{Align, Upright, text_on_line},
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

    let sql = r#"
        SELECT name, ST_LineMerge(ST_Collect(geometry)) AS geometry, type
          FROM osm_roads
          WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND name <> ''
          GROUP BY z_order, name, type
          ORDER BY z_order DESC"#;

    let buffer = ctx.meters_per_pixel() * 256.0;

    for row in &client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .unwrap()
    {
        let geom: LineString = row.get("geometry");

        let name: &str = row.get("name");

        context.set_source_color(colors::TRACK);

        text_on_line(
            ctx,
            geom.points.iter(),
            name,
            // "Idanská",
            // "Jánošíkova",
            // "Janosikova",
            Upright::Auto,
            Align::Center,
            Some(10.0),
            10.0,
        );
    }
}
