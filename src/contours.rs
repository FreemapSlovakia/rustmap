use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw_line,
};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, zoom: u32) {
    if zoom < 12 {
        return;
    }

    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        context,
        ..
    } = ctx;

    for row in &client.query(
        "SELECT wkb_geometry, height FROM contour_sk_split WHERE wkb_geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        &[&min_x, &min_y, &max_x, &max_y]
    ).unwrap() {
        let height: f64 = row.get("height");

        let width = match zoom {
            12 if height % 50.0 == 0.0 => 0.2,
            13..=14 if height % 100.0 == 0.0 => 0.4,
            13..=14 if height % 20.0 == 0.0 => 0.2,
            15.. if height % 100.0 == 0.0 => 0.6,
            15.. if height % 10.0 == 0.0 => 0.3,
            _ => 0.0,
        };

        if width == 0.0 {
            continue;
        }

        let geom: LineString = row.get("wkb_geometry");

        context.set_source_color_a(*colors::CONTOUR, 1.0 / 3.0);

        context.set_line_width(width);

        draw_line(&ctx, geom.points.iter());

        context.stroke().unwrap();
    }
}
