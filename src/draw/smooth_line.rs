use crate::{ctx::Ctx, point::Point};
use crate::draw::draw::{draw_line, Projectable};
use core::slice::Iter;
use postgis::ewkb::Point as PgPoint;

// https://github.com/ghaerr/agg-2.6/blob/master/agg-src/src/agg_vcgen_smooth_poly1.cpp
// https://agg.sourceforge.net/antigrain.com/research/bezier_interpolation/index.html
pub fn draw_smooth_bezier_spline(ctx: &Ctx, iter: Iter<PgPoint>, smooth_value: f64) {
    if smooth_value == 0.0 {
        draw_line(ctx, iter);

        return;
    }

    let mut points: Vec<Point> = iter.map(|p| p.project(ctx)).collect();

    let mut len = points.len();

    if len < 2 {
        panic!("At least two points are required");
    }

    let off = if points[0] == points[len - 1] {
        points.pop();

        len -= 1;

        1
    } else {
        0
    };

    if len < 2 {
        return;
    }

    let Point { x, y } = points[off];

    let context = &ctx.context;

    context.move_to(x, y);

    if len < 3 {
        let Point { x: x1, y: y1 } = points[1];

        context.line_to(x1, y1);

        return;
    }

    for i in off..len - 1 + off * 4 {
        let Point { x: x1, y: y1 } = points[i % len];
        let Point { x: x2, y: y2 } = points[(i + 1) % len];

        let len2 = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
        let xc2 = (x1 + x2) / 2.0;
        let yc2 = (y1 + y2) / 2.0;

        let ctrl1 = if off == 0 && i == 0 {
            Point::new(x1, y1)
        } else {
            let Point { x: x0, y: y0 } = points[(i - 1) % len];
            let len1 = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
            let k1 = len1 / (len1 + len2);
            let xc1 = (x0 + x1) / 2.0;
            let yc1 = (y0 + y1) / 2.0;
            let xm1 = xc1 + (xc2 - xc1) * k1;
            let ym1 = yc1 + (yc2 - yc1) * k1;

            Point::new(
                xm1 + (xc2 - xm1) * smooth_value + x1 - xm1,
                ym1 + (yc2 - ym1) * smooth_value + y1 - ym1,
            )
        };

        let ctrl2 = if off == 0 && i == len - 2 {
            Point::new(x2, y2)
        } else {
            let Point { x: x3, y: y3 } = points[(i + 2) % len];
            let len3 = ((x3 - x2).powi(2) + (y3 - y2).powi(2)).sqrt();
            let k2 = len2 / (len2 + len3);
            let xc3 = (x2 + x3) / 2.0;
            let yc3 = (y2 + y3) / 2.0;
            let xm2 = xc2 + (xc3 - xc2) * k2;
            let ym2 = yc2 + (yc3 - yc2) * k2;

            Point::new(
                xm2 + (xc2 - xm2) * smooth_value + x2 - xm2,
                ym2 + (yc2 - ym2) * smooth_value + y2 - ym2,
            )
        };

        context.curve_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, x2, y2);
    }
}
