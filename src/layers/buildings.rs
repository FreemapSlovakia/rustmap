use postgis::ewkb::Geometry;
use postgres::Client;

use crate::{bbox::BBox, ctx::Ctx, draw::draw::draw_geometry};

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

    context.save().expect("context saved");

    let sql = concat!(
        "SELECT type, geometry FROM osm_buildings ",
        "WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)"
    );

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y])
        .expect("db data");

    for row in rows {
        let geom: Geometry = row.get("geometry");

        context.set_source_rgb(0.5, 0.5, 0.5);

        draw_geometry(ctx, &geom);

        context.fill().unwrap();
    }

    context.restore().expect("context restored");
}
