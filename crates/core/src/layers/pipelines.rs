use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_line_string,
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

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

    let rows = client
        .query(&sql, &ctx.bbox_query_params(Some(8.0)).as_params())
        .expect("db data");

    context.save().expect("context saved");

    for row in rows {
        context.push_group();

        draw_line_string(
            context,
            &geometry_line_string(&row).project_to_tile(&ctx.tile_projector),
        );

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

    context.restore().expect("context restored");
}
