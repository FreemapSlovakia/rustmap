use postgis::ewkb::{LineString, Point};
use postgres::Client;
use crate::{
    bbox::BBox, colors::{self, ContextExt}, ctx::Ctx, draw::draw::{draw_line, Projectable}
};

pub fn render_lines(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: BBox { min_x, min_y, max_x, max_y },
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT geometry, type FROM osm_feature_lines WHERE {} AND geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        if zoom < 14 {
            "type = 'line'"
        } else {
            "type IN ('line', 'minor_line')"
        }
    );

    for row in &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: LineString = row.get("geometry");

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

        draw_line(ctx, geom.points.iter());

        context.stroke().unwrap();
    }
}

pub fn render_towers_poles(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: BBox { min_x, min_y, max_x, max_y },
        scale,
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = format!(
        "SELECT geometry, type
        FROM osm_features
        WHERE type IN ('tower'{}) AND geometry && make_buffered_envelope($1, $2, $3, $4, $5, 4)",
        if zoom < 15 { "" } else { ", 'pylon', 'pole'" }
    );

    for row in &client
        .query(&sql, &[min_x, min_y, max_x, max_y, &(zoom as i32)])
        .unwrap()
    {
        let geom: Point = row.get("geometry");

        context.set_source_color(if row.get::<_, &str>("type") == "pole" {
            colors::POWER_LINE_MINOR
        } else {
            colors::POWER_LINE
        });

        let p = geom.project(ctx);

        context.rectangle(
            ((p.x - 1.5) * scale).round() / scale,
            ((p.y - 1.5) * scale).round() / scale,
            3.0,
            3.0,
        );

        context.fill().unwrap();
    }
}
