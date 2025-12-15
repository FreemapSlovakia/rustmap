use postgres::Client;

use crate::{
    ctx::Ctx,
    draw::path_geom::path_geometry,
    projectable::{TileProjectable, geometry_geometry},
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let _span = tracy_client::span!("buildings::render");

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

        path_geometry(context, &geom);

        context.fill().unwrap();
    }

    context.restore().expect("context restored");
}
