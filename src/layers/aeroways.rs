use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_line,
};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let (way_width, dash_width, dash_array) = match zoom {
        11 => (3.0, 0.5, &[3.0, 3.0]),
        12..=13 => (5.0, 1.0, &[4.0, 4.0]),
        14.. => (8.0, 1.0, &[6.0, 6.0]),
        _ => panic!("unsupported zoom"),
    };

    let sql = "SELECT geometry, type FROM osm_aeroways WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)";

    for row in &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: LineString = row.get("geometry");

        draw_line(ctx, geom.points.iter());

        context.set_source_color(colors::AEROWAY);
        context.set_dash(&[], 0.0);
        context.set_line_width(way_width);
        context.stroke_preserve().unwrap();

        context.set_source_rgb(1.0, 1.0, 1.0);
        context.set_line_width(dash_width);
        context.set_dash(dash_array, 0.0);

        context.stroke().unwrap();
    }
}
