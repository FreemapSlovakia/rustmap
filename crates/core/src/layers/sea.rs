use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::path_geom::path_geometry,
    projectable::{TileProjectable, geometry_geometry},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let zoom = ctx.zoom;

    let sql = format!(
        "SELECT ST_Buffer(geometry, $5) AS geometry FROM {}
        WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        match zoom {
            ..=7 => "land_z5_7",
            8..=10 => "land_z8_10",
            11..=13 => "land_z11_13",
            14.. => "land_z14_plus",
        }
    );

    let mut params = ctx.bbox_query_params(None);

    params.push((20.0 - zoom as f64).exp2() / 25.0);

    let rows = client.query(&sql, &params.as_params()).expect("db data");

    for row in rows {
        let Some(geom) = geometry_geometry(&row) else {
            continue;
        };

        let geom = geom.project_to_tile(&ctx.tile_projector);

        path_geometry(context, &geom);

        context.set_source_color(colors::WHITE);
        context.fill().unwrap();
    }
}
