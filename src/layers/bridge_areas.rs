use postgis::ewkb::Geometry;
use postgres::Client;

use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_geometry,
};

pub fn render(ctx: &Ctx, client: &mut Client, mask: bool) {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        context,
        size: (width, height),
        ..
    } = ctx;

    let query = "SELECT geometry FROM osm_landusages WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857) AND type = 'bridge'";

    if mask {
        context.set_fill_rule(cairo::FillRule::EvenOdd);
    }

    for row in client.query(query, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: Geometry = row.get("geometry");

        if mask {
            context.rectangle(0.0, 0.0, *width as f64, *height as f64);
            draw_geometry(ctx, &geom);
            context.clip();
        } else {
            draw_geometry(ctx, &geom);
            context.set_source_color(*colors::INDUSTRIAL);
            context.fill_preserve().unwrap();

            context.set_line_width(1.0);
            context.set_dash(&[], 0.0);
            context.set_source_color(*colors::BUILDING);
            context.stroke().unwrap();
        }
    }
}
