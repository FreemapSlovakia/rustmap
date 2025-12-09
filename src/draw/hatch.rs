use crate::{
    ctx::Ctx,
    draw::draw::draw_geometry_uni,
    projectable::TileProjectable,
    xyz::{perpendicular_distance, to_absolute_pixel_coords},
};
use geo::{BoundingRect, Geometry, LineString};

pub fn hatch_geometry(ctx: &Ctx, geom: &Geometry, spacing: f64, angle: f64) {
    draw_geometry_uni(geom, &|iter| {
        hatch(ctx, iter, spacing, angle);
    });
}

pub fn hatch(ctx: &Ctx, line_string: &LineString, spacing: f64, angle: f64) {
    let projected = line_string.project_to_tile(&ctx.tile_projector);

    let Some(bounds) = projected.bounding_rect() else {
        return;
    };

    let Some(merc_bounds) = line_string.bounding_rect() else {
        return;
    };

    let center = merc_bounds.center();

    let (x, y) = to_absolute_pixel_coords(center.x, center.y, ctx.zoom as u8);

    let len = bounds.width().hypot(bounds.height()) / 2.0 + 1.0;

    let d = perpendicular_distance((0.0, 0.0), (x, y), angle) % spacing + 0.5;

    let context = ctx.context;

    context.save().unwrap();

    let center = bounds.center();

    context.translate(center.x, center.y);

    context.rotate(angle.to_radians());

    let mut off = 0.0;

    while off < len {
        context.move_to(-len, off + d);
        context.line_to(len, off + d);

        if off > 0.0 {
            context.move_to(-len, -off + d);
            context.line_to(len, -off + d);
        }

        off += spacing;
    }

    context.restore().unwrap();
}
