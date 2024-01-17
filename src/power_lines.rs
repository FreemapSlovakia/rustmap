use postgis::ewkb::{LineString, Point};
use postgres::Client;

use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{draw_line, Projectable},
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        scale,
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
                *colors::POWER_LINE
            } else {
                *colors::POWER_LINE_MINOR
            },
            0.5,
        );

        context.set_line_width(1.0);

        draw_line(ctx, geom.points.iter());

        context.stroke().unwrap();
    }

    if zoom < 14 {
        return;
    }

    let sql = format!(
        "SELECT geometry, type
        FROM osm_features
        WHERE type IN ('pylon', 'tower'{}) AND geometry && make_buffered_envelope($1, $2, $3, $4, $5, 4)",
        if zoom < 15 { "" } else { ", 'pole'" }
    );

    for row in &client.query(&sql, &[min_x, min_y, max_x, max_y, &zoom]).unwrap() {
        let geom: Point = row.get("geometry");

        context.set_source_color(if row.get::<_, &str>("type") == "pole" {
            *colors::POWER_LINE_MINOR
        } else {
            *colors::POWER_LINE
        });

        let (x, y) = geom.project(ctx);

        context.rectangle(
            ((x - 1.5) * scale).round() / scale,
            ((y - 1.5) * scale).round() / scale,
            3.0,
            3.0,
        );

        context.fill().unwrap();
    }
}
