use postgis::ewkb::Geometry;
use postgres::Client;

use crate::{
    bbox::BBox,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_geometry,
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
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = format!(
        "SELECT geometry, location IN('underground', 'underwater') AS below
        FROM osm_pipelines
        WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND location IN ({})",
        if zoom < 15 {
            "'overground', 'overhead', ''"
        } else {
            "'overground', 'overhead', '', 'underground', 'underwater'"
        }
    );

    let buffer = ctx.meters_per_pixel() * 8.0;

    let rows = client
        .query(&sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    for row in rows {
        let geom: Geometry = row.get("geometry");

        context.push_group();

        draw_geometry(ctx, &geom);

        context.set_source_color(colors::PIPELINE);
        context.set_dash(&[], 0.0);
        context.set_line_join(cairo::LineJoin::Round);
        context.set_line_width(2.0);

        context.stroke_preserve().unwrap();

        context.set_line_width(4.0);
        context.set_dash(&[0.0, 15.0, 1.5, 1.5, 1.5, 1.0], 0.0);

        context.stroke().unwrap();

        context.pop_group_to_source().unwrap();

        context
            .paint_with_alpha(if row.get("below") { 0.33 } else { 1.0 })
            .unwrap();
    }
}
