use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::path_geom::path_line_string,
    projectable::{TileProjectable, geometry_line_string, geometry_point},
};
use postgres::Client;

pub fn render_lines(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT geometry, type FROM osm_feature_lines WHERE {} AND geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        if zoom < 14 {
            "type = 'line'"
        } else {
            "type IN ('line', 'minor_line')"
        }
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(None).as_params())
        .expect("db data");

    for row in rows {
        context.set_source_color_a(
            if row.get::<_, &str>("type") == "line" {
                colors::POWER_LINE
            } else {
                colors::POWER_LINE_MINOR
            },
            0.5,
        );

        context.set_dash(&[], 0.0);
        context.set_line_width(1.0);

        path_line_string(
            context,
            &geometry_line_string(&row).project_to_tile(&ctx.tile_projector),
        );

        context.stroke().unwrap();
    }
}

pub fn render_towers_poles(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;
    let scale = ctx.scale;

    let zoom = ctx.zoom;

    let sql = format!(
        "SELECT geometry, type
        FROM osm_features
        WHERE type IN ('tower'{}) AND geometry && make_buffered_envelope($1, $2, $3, $4, $5, 4)",
        if zoom < 15 { "" } else { ", 'pylon', 'pole'" }
    );

    let mut params = ctx.bbox_query_params(None);
    params.push(zoom as i32);
    let query_params = params.as_params();

    let rows = client.query(&sql, &query_params).expect("db data");

    for row in rows {
        context.set_source_color(if row.get::<_, &str>("type") == "pole" {
            colors::POWER_LINE_MINOR
        } else {
            colors::POWER_LINE
        });

        let p = geometry_point(&row).project_to_tile(&ctx.tile_projector);

        context.rectangle(
            ((p.x() - 1.5) * scale).round() / scale,
            ((p.y() - 1.5) * scale).round() / scale,
            3.0,
            3.0,
        );

        context.fill().unwrap();
    }
}
