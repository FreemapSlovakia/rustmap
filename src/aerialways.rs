use crate::{ctx::Ctx, draw::draw_line};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let sql = "SELECT geometry, type FROM osm_aerialways WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)";

    for row in &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: LineString = row.get("geometry");

        context.set_source_rgb(0.0, 0.0, 0.0);
        context.set_dash(&[], 0.0);
        context.set_line_width(1.0);

        draw_line(ctx, geom.points.iter());

        context.stroke_preserve().unwrap();

        context.set_dash(&[1.0, 25.0], 0.0);
        context.set_line_width(5.0);

        context.stroke().unwrap();
    }
}
