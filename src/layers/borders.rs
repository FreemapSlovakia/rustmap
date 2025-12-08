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
        WITH segs AS (
            SELECT DISTINCT ON (m.member)
                m.member,
                m.geometry
            FROM osm_admin_members m
            JOIN osm_admin_relations r
                ON r.osm_id = m.osm_id
                AND r.admin_level = 2
            WHERE
                m.type = 1
                AND m.geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
        )
        SELECT ST_LineMerge(ST_Collect(geometry)) AS geometry
        FROM segs";

    let buffer = ctx.meters_per_pixel() * 10.0;

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    context.save().expect("context saved");

    ctx.context.push_group();

    for row in rows {
        let geom: Option<Geometry> = row.get("geometry");

        let Some(geometry) = geom else {
            continue;
        };

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

    context.pop_group_to_source().unwrap();
    context.paint_with_alpha(0.5).unwrap();
    context.restore().expect("context restored");
}
