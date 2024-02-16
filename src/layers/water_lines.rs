use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::smooth_line::draw_smooth_bezier_spline,
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

    let sql = &format!(
        "SELECT {}, type, seasonal OR intermittent AS tmp, tunnel FROM {} WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        match zoom {
            12 => "ST_Segmentize(ST_Simplify(geometry, 24), 200) AS geometry",
            13 => "ST_Segmentize(ST_Simplify(geometry, 12), 200) AS geometry",
            14 => "ST_Segmentize(ST_Simplify(geometry, 6), 200) AS geometry",
            _ => "geometry"
        },
        match zoom {
            ..=9 => "osm_waterways_gen0",
            10..=11 => "osm_waterways_gen1",
            _ => "osm_waterways"
        }
    );

    let rows = &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap();

    for pass in 0..=1 {
        let glow = pass == 0;

        for row in rows {
            let geom: LineString = row.get("geometry");

            let typ: &str = row.get("type");

            context.set_dash(if row.get("tmp") { &[6.0, 3.0] } else { &[] }, 0.0);

            let (width, smooth) = match (typ, zoom) {
                ("river", ..=8) => (1.5f64.powf(*zoom as f64 - 8.0), 0.0),
                ("river", 9) => (1.5, 0.0),
                ("river", 10..=11) => (2.2, 0.0),
                ("river", 12..) => (2.2, 0.5),
                (_, 12..) if typ != "river" => (if *zoom == 12 { 1.0 } else { 1.2 }, 0.5),
                _ => (0.0, 0.0), // TODO panic?
            };

            if glow {
                if *zoom >= 12 {
                    context.set_source_color(*colors::WATER);

                    context.set_source_rgba(
                        1.0,
                        1.0,
                        1.0,
                        if row.get("tunnel") { 0.8 } else { 0.5 },
                    );

                    context.set_line_width(if typ == "river" {
                        3.4
                    } else if *zoom == 12 {
                        2.0
                    } else {
                        2.4
                    });

                    draw_smooth_bezier_spline(ctx, geom.points.iter(), smooth);

                    context.stroke().unwrap();
                }
            } else {
                context
                    .set_source_color_a(*colors::WATER, if row.get("tunnel") { 0.33 } else { 1.0 });

                context.set_line_width(width);

                draw_smooth_bezier_spline(ctx, geom.points.iter(), smooth);

                context.stroke().unwrap();
            }
        }
    }
}
