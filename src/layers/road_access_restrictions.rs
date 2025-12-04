use postgis::ewkb::Geometry;
use postgres::Client;
use std::cell::Cell;

use crate::{
    bbox::BBox,
    ctx::Ctx,
    draw::{draw::draw_geometry, markers_on_path::draw_markers_on_path},
};

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        ..
    } = ctx;

    let sql = "
        SELECT
            CASE
            WHEN bicycle NOT IN ('', 'yes', 'designated', 'official', 'permissive')
            OR bicycle = '' AND vehicle NOT IN ('', 'yes', 'designated', 'official', 'permissive')
            OR bicycle = '' AND vehicle = '' AND access NOT IN ('', 'yes', 'designated', 'official', 'permissive')
            THEN 1 ELSE 0 END AS no_bicycle,
            CASE
            WHEN foot NOT IN ('', 'yes', 'designated', 'official', 'permissive')
            OR foot = '' AND access NOT IN ('', 'yes', 'designated', 'official', 'permissive')
            THEN 1 ELSE 0 END AS no_foot,
            geometry
        FROM osm_roads
        WHERE
            type NOT IN ('trunk', 'motorway', 'trunk_link', 'motorway_link')
                AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)";

    let buffer = ctx.meters_per_pixel() * 32.0;

    // TODO lazy

    let no_bicycle_icon = &ctx
        .svg_cache
        .borrow_mut()
        .get("images/no_bicycle.svg")
        .clone();

    let no_foot_icon = &ctx.svg_cache.borrow_mut().get("images/no_foot.svg").clone();

    let no_bicycle_rect = no_bicycle_icon.extents().unwrap();
    let no_foot_rect = no_foot_icon.extents().unwrap();

    for row in &client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .unwrap()
    {
        let geom: Geometry = row.get("geometry");

        draw_geometry(ctx, &geom);

        let path = context.copy_path_flat().unwrap();

        context.new_path();

        let no_bicycle = row.get::<_, i32>("no_bicycle") > 0;
        let no_foot = row.get::<_, i32>("no_foot") > 0;

        if !no_bicycle && !no_foot {
            continue;
        }

        let i_cell = Cell::new(0);

        draw_markers_on_path(&path, 12.0, 24.0, &|x, y, angle| {
            let i = i_cell.get();

            let (arrow, rect) = if no_bicycle && no_foot && i % 2 == 0 {
                (no_bicycle_icon, no_bicycle_rect)
            } else if no_foot {
                (no_foot_icon, no_foot_rect)
            } else {
                (no_bicycle_icon, no_bicycle_rect)
            };

            context.save().unwrap();
            context.translate(x, y);
            context.rotate(angle);
            context
                .set_source_surface(arrow, -rect.width() / 2.0, -rect.height() / 2.0)
                .unwrap();
            context.paint_with_alpha(0.75).unwrap();
            context.restore().unwrap();

            i_cell.set(i + 1);
        });
    }
}
