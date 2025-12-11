use postgres::Client;

use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{draw::draw_geometry, hatch::hatch_geometry},
    projectable::{TileProjectable, geometry_geometry},
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let zoom = ctx.zoom;

    let d = 4.0f64.max(1.33f64.powf(zoom as f64) / 10.0).round();

    let sql = concat!(
        "SELECT geometry FROM osm_power_generators ",
        "WHERE source = 'solar' AND geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)"
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(None).as_params())
        .expect("db data");

    for row in rows {
        let Some(geom) = geometry_geometry(&row) else {
            continue;
        };

        context.push_group();

        let projected = geom.project_to_tile(&ctx.tile_projector);

        draw_geometry(context, &projected);

        context.clip();

        context.set_source_color(colors::SOLAR_BG);
        context.paint().unwrap();

        hatch_geometry(ctx, &geom, d, 0.0);
        hatch_geometry(ctx, &geom, d, 90.0);

        context.set_source_color(colors::SOLAR_FG);
        context.set_dash(&[], 0.0);
        context.set_line_width(1.0);
        context.stroke().unwrap();

        context.pop_group_to_source().unwrap();
        context.paint().unwrap();
    }
}
