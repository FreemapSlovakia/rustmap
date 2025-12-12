use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{hatch::hatch_geometry, path_geom::path_geometry},
    projectable::{TileProjectable, geometry_geometry},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let zoom = ctx.zoom;

    let sql = "
        SELECT geometry
            FROM osm_landusages
            WHERE
                type = 'military'
                AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
                AND area / POWER(4, 19 - $6) > 10";

    let mut params = ctx.bbox_query_params(Some(10.0));
    params.push(zoom as i32);

    let rows = &client.query(sql, &params.as_params()).expect("db data");

    ctx.context.push_group();

    ctx.context.push_group();

    let geometries: Vec<_> = rows
        .iter()
        .filter_map(geometry_geometry)
        .map(|geom| (geom.project_to_tile(&ctx.tile_projector), geom))
        .collect();

    // hatching
    for (projected, unprojected) in &geometries {
        ctx.context.push_group();

        path_geometry(context, projected);

        context.clip();

        hatch_geometry(ctx, unprojected, 10.0, -45.0);

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

    for (projected, _) in &geometries {
        ctx.context.set_source_color(colors::MILITARY);
        ctx.context.set_dash(&[25.0, 7.0], 0.0);
        ctx.context.set_line_width(3.0);
        path_geometry(context, projected);
        ctx.context.stroke().unwrap();
    }

    ctx.context.pop_group_to_source().unwrap();

    ctx.context.paint_with_alpha(0.8).unwrap();
}
