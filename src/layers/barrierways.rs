use postgis::ewkb::LineString;
use postgres::Client;

use crate::{
    bbox::BBox,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_line,
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        zoom,
        ..
    } = ctx;

    let sql = concat!(
        "SELECT geometry, type FROM osm_barrierways ",
        "WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"
    );

    let buffer = ctx.meters_per_pixel() * 8.0;

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    for row in rows {
        let geom: LineString = row.get("geometry");

        context.save().expect("context saved");

        match row.get("type") {
            "city_wall" => {
                context.set_dash(&[], 0.0);
                context.set_source_color(colors::BUILDING);
                context.set_line_width(2.0);
            }
            "hedge" => {
                context.set_source_color(colors::PITCH);
                context.set_line_width(*zoom as f64 - 14.0);
                context.set_dash(&[0.01, *zoom as f64 - 14.0], 0.0);
                context.set_line_join(cairo::LineJoin::Round);
                context.set_line_cap(cairo::LineCap::Round);
            }
            _ => {
                context.set_dash(&[2.0, 1.0], 0.0);
                context.set_line_width(1.0);
                context.set_source_color(colors::BARRIERWAY);
            }
        }

        draw_line(ctx, geom.points.iter());

        context.stroke().unwrap();

        context.restore().expect("context restores");
    }
}
