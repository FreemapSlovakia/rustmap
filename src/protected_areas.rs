use postgis::ewkb::{Geometry, Point};
use postgres::Client;
use crate::{
    ctx::Ctx, draw::{draw_mpoly, draw_mpoly_uni}, line_pattern::draw_line_pattern
};
use core::slice::Iter;

fn draw_protected_area_border(ctx: &Ctx, iter: Iter<Point>) {
    draw_line_pattern(ctx, iter, 0.8, "images/protected_area.svg");
}

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT type, protect_class, geometry FROM osm_protected_areas WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        // if zoom < 14 {
        //     "type = 'line'"
        // } else {
        //     "type IN ('line', 'minor_line')"
        // }
    );

    for row in &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: Geometry = row.get("geometry");
        let typ: &str = row.get("type");
        let protect_class: &str = row.get("protect_class");

        if zoom >= 8 && zoom <= 11 && (typ == "national_park" || typ == "protected_area" && protect_class == "2") {
            // <PolygonPatternSymbolizer
            //     file="images/national_park_area.svg"
            //     alignment="global"
            //     opacity="0.4 + ((@zoom < 11) * 0.1)"
            // />
        }

        if zoom >= 12 && (typ == "nature_reserve" || typ == "protected_area" && protect_class != "2") {
            // draw_mpoly(ctx, &geom);
            // ctx.context.set_source_rgb(0.0, 0.0, 0.0);
            // ctx.context.set_dash(&[], 0.0);
            // ctx.context.set_line_width(1.0);

            // ctx.context.stroke().unwrap();

            draw_mpoly_uni(ctx, &geom, &draw_protected_area_border);
        }
    }
}
