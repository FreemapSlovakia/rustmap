use crate::ctx::Ctx;
use postgis::ewkb::{self, Geometry, Point, Polygon};

pub fn draw_mpoly(geom: ewkb::GeometryT<Point>, ctx: &Ctx) {
    match geom {
        Geometry::Polygon(p) => {
            draw_poly(ctx, &p);
        }
        Geometry::MultiPolygon(p) => {
            for poly in &p.polygons {
                draw_poly(ctx, poly);
            }
        }
        _ => {
            println!("?");
        }
    }
}

pub fn draw_line(ctx: &Ctx, iter: core::slice::Iter<Point>) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        size: (w_i, h_i),
    } = ctx;

    let w = *w_i as f64;
    let h = *h_i as f64;

    for (i, p) in iter.enumerate() {
        let x = ((p.x - min_x) / (max_x - min_x)) * w;

        let y = h - ((p.y - min_y) / (max_y - min_y)) * h;

        if i == 0 {
            context.move_to(x, y);
        } else {
            context.line_to(x, y);
        }
    }
}

pub fn draw_poly(ctx: &Ctx, poly: &Polygon) {
    let context = &ctx.context;

    for ring in &poly.rings {
        draw_line(&ctx, ring.points.iter());

        // ring.points
        // context.close_path();
    }

    context.fill().unwrap();
}
