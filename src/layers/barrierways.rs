use postgis::ewkb::LineString;
use postgres::Client;

use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_line,
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        zoom,
        ..
    } = ctx;

    for row in &client.query("SELECT geometry, type FROM osm_barrierways WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)", &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: LineString = row.get("geometry");

        context.save().unwrap();

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

        context.restore().unwrap();

    }
}
