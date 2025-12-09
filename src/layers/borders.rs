use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_geometry,
    projectable::{TileProjectable, geometry_geometry},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

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

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(10.0)).as_params())
        .expect("db data");

    context.save().expect("context saved");

    ctx.context.push_group();

    for row in rows {
        let Some(geometry) =
            geometry_geometry(&row).map(|geom| geom.project_to_tile(&ctx.tile_projector))
        else {
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
        draw_geometry(context, &geometry);
        ctx.context.stroke().unwrap();
    }

    context.pop_group_to_source().unwrap();
    context.paint_with_alpha(0.5).unwrap();
    context.restore().expect("context restored");
}
