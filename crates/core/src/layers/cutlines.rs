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

    let sql = concat!(
        "SELECT geometry FROM osm_feature_lines ",
        "WHERE type = 'cutline' AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(8.0)).as_params())
        .expect("db data");

    context.save().expect("context saved");

    for row in rows {
        draw_line_string(
            context,
            &geometry_line_string(&row).project_to_tile(&ctx.tile_projector),
        );

        context.set_source_color(colors::SCRUB);
        context.set_dash(&[], 0.0);
        context.set_line_width(2.0 + 0.33 * 2f64.powf((zoom - 12) as f64));
        context.stroke_preserve().unwrap();
        context.stroke().unwrap();
    }

    context.restore().expect("context restored");
}
