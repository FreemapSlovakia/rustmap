use crate::{
    bbox::BBox,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_geometry,
};
use postgis::ewkb::Geometry;
use postgres::Client;

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

    let sql = "
        SELECT ST_LineMerge(ST_Collect(geometry)) AS geometry
        FROM osm_admin
        WHERE admin_level = 2 AND geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
    ";

    let rows = &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap();

    ctx.context.push_group();

    for row in rows {
        let geom: Option<Geometry> = row.get("geometry");

        if let Some(geometry) = geom {
            ctx.context.set_dash(&[], 0.0);
            ctx.context.set_source_color(colors::ADMIN_BORDER);
            ctx.context.set_line_width(if zoom <= 10 {
                0.5 + 6.0 * 1.4f64.powf(zoom as f64 - 11.0)
            } else {
                6.0
            });
            ctx.context.set_line_join(cairo::LineJoin::Round);
            draw_geometry(ctx, &geometry);
            ctx.context.stroke().unwrap();
        }
    }

    context.pop_group_to_source().unwrap();
    context.paint_with_alpha(0.5).unwrap();
}
