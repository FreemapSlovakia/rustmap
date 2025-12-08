use crate::{
    ctx::Ctx,
    draw::draw::draw_geometry_uni,
    projectable::Projectable,
    xyz::{perpendicular_distance, to_absolute_pixel_coords},
};
use geo::Coord;
use postgis::ewkb::{Geometry, Point};
use std::borrow::Borrow;

pub fn hatch_geometry(ctx: &Ctx, geom: &Geometry, spacing: f64, angle: f64) {
    draw_geometry_uni(geom, &|iter| {
        hatch(ctx, iter, spacing, angle);
    });
}

pub fn hatch<P>(ctx: &Ctx, points: impl IntoIterator<Item = P>, spacing: f64, angle: f64)
where
    P: Borrow<Point>,
{
    let mut merc_min_x = f64::INFINITY;
    let mut merc_max_x = f64::NEG_INFINITY;
    let mut merc_min_y = f64::INFINITY;
    let mut merc_max_y = f64::NEG_INFINITY;

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in points {
        let p = p.borrow();

        merc_min_x = merc_min_x.min(p.x);
        merc_max_x = merc_max_x.max(p.x);
        merc_min_y = merc_min_y.min(p.y);
        merc_max_y = merc_max_y.max(p.y);

        let Coord { x, y } = p.project(ctx);

        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    let (x, y) = to_absolute_pixel_coords(
        (merc_max_x + merc_min_x) / 2.0,
        (merc_max_y + merc_min_y) / 2.0,
        ctx.zoom as u8,
    );

    let len = (max_x - min_x).hypot(max_y - min_y) / 2.0 + 1.0;

    let d = perpendicular_distance((0.0, 0.0), (x, y), angle) % spacing + 0.5;

    ctx.context.save().unwrap();

    ctx.context
        .translate((max_x + min_x) / 2.0, (max_y + min_y) / 2.0);

    ctx.context.rotate(angle.to_radians());

    let mut off = 0.0;

    while off < len {
        ctx.context.move_to(-len, off + d);
        ctx.context.line_to(len, off + d);

        if off > 0.0 {
            ctx.context.move_to(-len, -off + d);
            ctx.context.line_to(len, -off + d);
        }

        off += spacing;
    }

    ctx.context.restore().unwrap();
}
