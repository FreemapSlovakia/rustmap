use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::draw::draw_geometry,
    projectable::{TileProjectable, geometry_geometry},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    for row in &client.query(
        "SELECT type, geometry FROM osm_waterareas WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        &ctx.bbox_query_params(None).as_params()

    ).expect("db data") {
        let Some(geom) =
            geometry_geometry(&row).map(|geom| geom.project_to_tile(&ctx.tile_projector))
        else {
            continue;
        };

        context.set_source_color(colors::WATER);

        draw_geometry(context, &geom);

        context.fill().unwrap();
  }
}
