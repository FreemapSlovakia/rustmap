use crate::{
    SvgCache,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{markers_on_path::draw_markers_on_path, smooth_line::path_smooth_bezier_spline},
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, svg_cache: &mut SvgCache) {
    let _span = tracy_client::span!("water_lines::render");

    let context = ctx.context;
    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT {}, type, seasonal OR intermittent AS tmp, tunnel FROM {} WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)",
        match zoom {
            12 => "ST_Segmentize(ST_Simplify(geometry, 24), 200) AS geometry",
            13 => "ST_Segmentize(ST_Simplify(geometry, 12), 200) AS geometry",
            14 => "ST_Segmentize(ST_Simplify(geometry, 6), 200) AS geometry",
            _ => "geometry",
        },
        match zoom {
            ..=9 => "osm_waterways_gen0",
            10..=11 => "osm_waterways_gen1",
            _ => "osm_waterways",
        }
    );

    let rows = &client
        .query(sql, &ctx.bbox_query_params(Some(8.0)).as_params())
        .expect("db data");

    // TODO lazy
    let arrow = svg_cache.get("waterway-arrow.svg");

    let (dx, dy) = {
        let rect = arrow.extents().unwrap();

        (-rect.width() / 2.0, -rect.height() / 2.0)
    };

    context.save().expect("context saved");

    for pass in 0..=1 {
        let glow = pass == 0;

        for row in rows {
            let geom = geometry_line_string(row).project_to_tile(&ctx.tile_projector);

            let typ: &str = row.get("type");

            context.set_dash(if row.get("tmp") { &[6.0, 3.0] } else { &[] }, 0.0);

            let (width, smooth) = match (typ, zoom) {
                ("river", ..=8) => (1.5f64.powf(zoom as f64 - 8.0), 0.0),
                ("river", 9) => (1.5, 0.0),
                ("river", 10..=11) => (2.2, 0.0),
                ("river", 12..) => (2.2, 0.5),
                (_, 12..) if typ != "river" => (if zoom == 12 { 1.0 } else { 1.2 }, 0.5),
                _ => (0.0, 0.0), // TODO panic?
            };

            if glow {
                if zoom >= 12 {
                    context.set_source_color(colors::WATER);

                    context.set_source_rgba(
                        1.0,
                        1.0,
                        1.0,
                        if row.get("tunnel") { 0.8 } else { 0.5 },
                    );

                    context.set_line_width(if typ == "river" {
                        3.4
                    } else if zoom == 12 {
                        2.0
                    } else {
                        2.4
                    });

                    path_smooth_bezier_spline(context, &geom, smooth);

                    context.stroke().unwrap();
                }
            } else {
                context
                    .set_source_color_a(colors::WATER, if row.get("tunnel") { 0.33 } else { 1.0 });

                context.set_line_width(width);

                path_smooth_bezier_spline(context, &geom, smooth);

                let path = context.copy_path_flat().unwrap();

                context.stroke().unwrap();

                draw_markers_on_path(&path, 150.0, 300.0, &|x, y, angle| {
                    context.save().unwrap();
                    context.translate(x, y);
                    context.rotate(angle);
                    context.set_source_surface(arrow, dx, dy).unwrap();
                    context.paint().unwrap();
                    context.restore().unwrap();
                });
            }
        }
    }

    context.restore().expect("context restored");
}
