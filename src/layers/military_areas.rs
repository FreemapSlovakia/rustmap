use crate::{
    bbox::BBox, colors::{self, ContextExt}, ctx::Ctx, draw::{draw::draw_geometry, hatch::hatch_geometry}
};
use postgis::ewkb::Geometry;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: BBox { min_x, min_y, max_x, max_y },
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = "
        SELECT geometry
            FROM osm_landusages
            WHERE
                type = 'military'
                AND geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
                AND area / POWER(4, 19 - $5) > 10";

    let rows = &client
        .query(sql, &[min_x, min_y, max_x, max_y, &(zoom as i32)])
        .unwrap();

    ctx.context.push_group();

    ctx.context.push_group();

    // hatching
    for row in rows {
        let geom: Geometry = row.get("geometry");

        ctx.context.push_group();

        draw_geometry(ctx, &geom);

        context.clip();

        hatch_geometry(ctx, &geom, 10.0, -45.0);

        ctx.context.set_source_color(colors::MILITARY);
        ctx.context.set_dash(&[], 0.0);
        ctx.context.set_line_width(1.5);
        ctx.context.stroke().unwrap();

        context.pop_group_to_source().unwrap();
        context.paint().unwrap();
    }

    context.pop_group_to_source().unwrap();
    context
        .paint_with_alpha(if zoom < 14 { 0.5 / 0.8 } else { 0.2 / 0.8 })
        .unwrap();

    // border

    for row in rows {
        let geom: Geometry = row.get("geometry");

        ctx.context.set_source_color(colors::MILITARY);
        ctx.context.set_dash(&[25.0, 7.0], 0.0);
        ctx.context.set_line_width(3.0);
        draw_geometry(ctx, &geom);
        ctx.context.stroke().unwrap();
    }

    ctx.context.pop_group_to_source().unwrap();

    ctx.context.paint_with_alpha(0.8).unwrap();
}
