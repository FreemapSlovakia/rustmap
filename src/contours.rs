use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw_smooth_bezier_spline,
};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        context,
        zoom,
        ..
    } = ctx;

    if *zoom < 12 {
        return;
    }


    let simplify_factor: f64 = match zoom { ..=12 => 2000.0, 13 => 1000.0, 14 => 200.0, 15 => 50.0, _ => 0.0 };

    // TODO measure performance impact of simplification, if it makes something faster

    for row in &client.query(
        "SELECT ST_SimplifyVW((ST_Dump(ST_LineMerge(ST_Collect(ST_ClipByBox2D(wkb_geometry, ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), 100)))))).geom, $5) AS geom, height
        FROM contour_sk_split
        WHERE wkb_geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
        GROUP BY height",
        &[min_x, min_y, max_x, max_y, &simplify_factor]
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

        let geom: LineString = row.get("geom");

        context.set_dash(&[], 0.0);

        context.set_line_width(width);

        context.set_source_color_a(*colors::CONTOUR, 1.0 / 3.0);

        // draw_line(&ctx, geom.points.iter());

        // context.stroke().unwrap();

        draw_smooth_bezier_spline(&ctx, geom.points.iter(), 1.0);

        context.stroke().unwrap();
    }
}
