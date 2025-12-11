use postgres::Client;

use crate::{
    ctx::Ctx,
    draw::draw::draw_geometry,
    projectable::{TileProjectable, geometry_geometry},
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    context.save().expect("context saved");

    let sql = concat!(
        "SELECT type, geometry FROM osm_buildings ",
        "WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)"
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(None).as_params())
        .expect("db data");

    for row in rows {
        let Some(geom) =
            geometry_geometry(&row).map(|geom| geom.project_to_tile(&ctx.tile_projector))
        else {
            continue;
        };

        context.set_source_rgb(0.5, 0.5, 0.5);

        draw_geometry(context, &geom);

        context.fill().unwrap();
    }

    context.restore().expect("context restored");
}
