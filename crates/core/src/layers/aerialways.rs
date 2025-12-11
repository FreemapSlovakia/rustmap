use crate::{
    ctx::Ctx,
    draw::draw::draw_line,
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let sql = concat!(
        "SELECT geometry, type FROM osm_aerialways ",
        "WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(10.0)).as_params())
        .expect("db data");

    context.save().expect("context saved");

    for row in rows {
        context.set_source_rgb(0.0, 0.0, 0.0);
        context.set_dash(&[], 0.0);
        context.set_line_width(1.0);

        draw_line(
            context,
            &geometry_line_string(&row).project_to_tile(&ctx.tile_projector),
        );

        context.stroke_preserve().unwrap();

        context.set_dash(&[1.0, 25.0], 0.0);
        context.set_line_width(5.0);

        context.stroke().unwrap();
    }

    context.restore().expect("context restored");
}
