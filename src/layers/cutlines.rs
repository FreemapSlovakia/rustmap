use crate::{
    bbox::BBox,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_line,
};
use postgis::ewkb::LineString;
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

    let sql = concat!(
        "SELECT geometry FROM osm_feature_lines ",
        "WHERE type = 'cutline' AND geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)"
    );

    context.save().expect("context saved");

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y])
        .expect("db data");

    for row in rows {
        let geom: LineString = row.get("geometry");

        draw_line(ctx, geom.points.iter());

        context.set_source_color(colors::SCRUB);
        context.set_dash(&[], 0.0);
        context.set_line_width(2.0 + 0.33 * 2f64.powf((zoom - 12) as f64));
        context.stroke_preserve().unwrap();

        context.stroke().unwrap();
    }

    context.restore().expect("context restored");
}
