use crate::{
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{draw_line_off, draw_mpoly, draw_mpoly_uni},
    hatch::hatch_geometry,
    line_pattern::draw_line_pattern,
};
use core::slice::Iter;
use postgis::ewkb::{Geometry, Point};
use postgres::Client;

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
        "SELECT type, protect_class, geometry FROM osm_protected_areas WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857) {}",
        if zoom < 12 { " AND NOT (type = 'nature_reserve' OR type = 'protected_area' AND protect_class <> '2')" } else { "" }
    );

    let rows = &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap();

    // hatching
    if zoom <= 11 {
        for row in rows {
            let typ: &str = row.get("type");
            let protect_class: &str = row.get("protect_class");
            let geom: Geometry = row.get("geometry");

            if typ == "national_park" || typ == "protected_area" && protect_class == "2" {
                ctx.context.push_group();

                draw_mpoly(ctx, &geom);

                context.clip();

                hatch_geometry(ctx, &geom, 3.0, -45.0);

                ctx.context
                    .set_source_color_a(*colors::PROTECTED, if zoom < 11 { 0.5 } else { 0.4 });
                ctx.context.set_dash(&[], 0.0);
                ctx.context.set_line_width(0.7);
                ctx.context.stroke().unwrap();

                context.pop_group_to_source().unwrap();
                context.paint().unwrap();
            }
        }
    }

    // border
    for row in rows {
        let typ: &str = row.get("type");
        let protect_class: &str = row.get("protect_class");
        let geom: Geometry = row.get("geometry");

        if typ == "nature_reserve" || typ == "protected_area" && protect_class != "2" {
            draw_mpoly_uni(&geom, |iter| draw_protected_area_border(ctx, iter));
        }
    }

    ctx.context.push_group();

    for row in rows {
        let typ: &str = row.get("type");
        let protect_class: &str = row.get("protect_class");
        let geom: Geometry = row.get("geometry");

        if typ == "national_park" || typ == "protected_area" && protect_class == "2" {
            let wb = if zoom > 10 {
                0.5 * (zoom as f64 - 10.0) + 2.0
            } else {
                2.0
            };

            ctx.context.set_source_color(*colors::PROTECTED);
            ctx.context.set_dash(&[], 0.0);
            ctx.context.set_line_width(wb * 0.75);
            ctx.context.set_line_join(cairo::LineJoin::Round);
            draw_mpoly(ctx, &geom);
            ctx.context.stroke().unwrap();

            ctx.context.set_line_width(wb);
            ctx.context.set_source_color_a(*colors::PROTECTED, 0.5);
            draw_mpoly_uni(&geom, |iter| draw_line_off(ctx, iter, wb * 0.75));
            ctx.context.stroke().unwrap();
        }
    }

    ctx.context.pop_group_to_source().unwrap();

    ctx.context.paint_with_alpha(0.66).unwrap();
}
