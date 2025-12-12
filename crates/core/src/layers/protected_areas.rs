use crate::{
    SvgCache,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{
        draw::{draw_geometry, draw_geometry_uni, draw_line_string_with_offset},
        hatch::hatch_geometry,
        line_pattern::draw_line_pattern,
    },
    projectable::{TileProjectable, geometry_geometry},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, svg_cache: &mut SvgCache) {
    let context = ctx.context;

    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT type, protect_class, geometry FROM osm_protected_areas WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) {}",
        if zoom < 12 {
            " AND NOT (type = 'nature_reserve' OR type = 'protected_area' AND protect_class <> '2')"
        } else {
            ""
        }
    );

    let rows = &client
        .query(sql, &ctx.bbox_query_params(Some(10.0)).as_params())
        .expect("db data");

    let geometries: Vec<_> = rows
        .iter()
        .filter_map(|row| {
            geometry_geometry(row)
                .map(|geom| (geom.project_to_tile(&ctx.tile_projector), geom, row))
        })
        .collect();

    context.save().expect("context saved");

    // hatching
    if zoom <= 11 {
        for (projected, unprojected, row) in &geometries {
            let typ: &str = row.get("type");
            let protect_class: &str = row.get("protect_class");

            if typ == "national_park" || typ == "protected_area" && protect_class == "2" {
                context.push_group();

                draw_geometry(context, projected);

                context.clip();

                hatch_geometry(ctx, unprojected, 3.0, -45.0);

                context.set_source_color_a(colors::PROTECTED, if zoom < 11 { 0.5 } else { 0.4 });
                context.set_dash(&[], 0.0);
                context.set_line_width(0.7);
                context.stroke().unwrap();

                context.pop_group_to_source().unwrap();
                context.paint().unwrap();
            }
        }
    }

    // border
    for (projected, _, row) in &geometries {
        let typ: &str = row.get("type");
        let protect_class: &str = row.get("protect_class");

        if typ == "nature_reserve" || typ == "protected_area" && protect_class != "2" {
            let sample = svg_cache.get("protected_area.svg");

            draw_geometry_uni(projected, &|line_string| {
                draw_line_pattern(ctx, line_string, 0.8, sample)
            });
        }
    }

    context.push_group();

    for (projected, _, row) in &geometries {
        let typ: &str = row.get("type");
        let protect_class: &str = row.get("protect_class");

        if typ == "national_park" || typ == "protected_area" && protect_class == "2" {
            let wb = if zoom > 10 {
                0.5f64.mul_add(zoom as f64 - 10.0, 2.0)
            } else {
                2.0
            };

            context.set_source_color(colors::PROTECTED);
            context.set_dash(&[], 0.0);
            context.set_line_width(wb * 0.75);
            context.set_line_join(cairo::LineJoin::Round);
            draw_geometry(context, projected);
            context.stroke().unwrap();

            context.set_line_width(wb);
            context.set_source_color_a(colors::PROTECTED, 0.5);
            draw_geometry_uni(projected, &|iter| {
                draw_line_string_with_offset(context, iter, wb * 0.75)
            });
            context.stroke().unwrap();
        }
    }

    context.pop_group_to_source().unwrap();

    context.paint_with_alpha(0.66).unwrap();

    context.restore().expect("context restored");
}
