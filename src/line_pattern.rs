use crate::ctx::Ctx;
use crate::draw::Projectable;
use cairo::{Matrix, SurfacePattern};
use core::slice::Iter;

type Point = (f64, f64);

pub fn draw_line_pattern(ctx: &Ctx, iter: Iter<postgis::ewkb::Point>, miter_limit: f64, image: &str) {
    let pts: Vec<Point> = iter.map(|p| p.project(ctx)).collect();

    draw_polyline_outline(ctx, &pts[..], miter_limit, image);
}

fn get_perpendicular(dx: f64, dy: f64, length: f64, stroke_width: f64) -> Point {
    (
        (-dy / length) * stroke_width / 2.0,
        (dx / length) * stroke_width / 2.0,
    )
}

fn get_intersection(p1: Point, p2: Point, p3: Point, p4: Point) -> Option<Point> {
    let s1_x = p2.0 - p1.0;
    let s1_y = p2.1 - p1.1;
    let s2_x = p4.0 - p3.0;
    let s2_y = p4.1 - p3.1;

    let denom = s1_x * s2_y - s2_x * s1_y;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let s = (-s1_y * (p1.0 - p3.0) + s1_x * (p1.1 - p3.1)) / denom;
    Some((p3.0 + s * s2_x, p3.1 + s * s2_y))
}

fn should_use_bevel_join(
    p0: Point,
    p1: Point,
    p2: Point,
    stroke_width: f64,
    miter_limit: f64,
) -> bool {
    let v1 = (p1.0 - p0.0, p1.1 - p0.1);
    let v2 = (p2.0 - p1.0, p2.1 - p1.1);

    let len_v1 = (v1.0.powi(2) + v1.1.powi(2)).sqrt();
    let len_v2 = (v2.0.powi(2) + v2.1.powi(2)).sqrt();

    let v1_norm = (v1.0 / len_v1, v1.1 / len_v1);
    let v2_norm = (v2.0 / len_v2, v2.1 / len_v2);

    let dot = v1_norm.0 * v2_norm.0 + v1_norm.1 * v2_norm.1;
    let angle = dot.clamp(-1.0, 1.0).acos();

    let miter_length = stroke_width / (2.0 * (angle / 2.0).sin());

    miter_length > miter_limit * stroke_width
}

fn cross_product(v1: Point, v2: Point, v3: Point) -> f64 {
    let vector_a = (v2.0 - v1.0, v2.1 - v1.1);
    let vector_b = (v3.0 - v2.0, v3.1 - v2.1);
    vector_a.0 * vector_b.1 - vector_a.1 * vector_b.0
}

fn compute_corners(p0: Point, p1: Point, stroke_width: f64) -> (Point, Point, Point, Point) {
    let dx0 = p1.0 - p0.0;
    let dy0 = p1.1 - p0.1;
    let length0 = (dx0.powi(2) + dy0.powi(2)).sqrt();
    let perp0 = get_perpendicular(dx0, dy0, length0, stroke_width);

    (
        (p0.0 + perp0.0, p0.1 + perp0.1),
        (p0.0 - perp0.0, p0.1 - perp0.1),
        (p1.0 - perp0.0, p1.1 - perp0.1),
        (p1.0 + perp0.0, p1.1 + perp0.1),
    )
}

// Assuming type Point and other functions are defined

pub fn draw_polyline_outline(ctx: &Ctx, vertices: &[Point], miter_limit: f64, image: &str) {
    if vertices.len() < 2 {
        return;
    }

    let mut cache = ctx.cache.borrow_mut();

    let tile = cache.get_svg(image);

    let pattern = SurfacePattern::create(tile);

    let rect = tile.extents().unwrap();

    let stroke_width = rect.height();

    let context = &ctx.context;

    let mut dist = 0.0;

    for i in 0..vertices.len() - 1 {
        let p1 = vertices[i];
        let p2 = vertices[i + 1];
        let (mut corner1, mut corner2, mut corner3, mut corner4) =
            compute_corners(p1, p2, stroke_width);
        let mut extra_corner1: Option<Point> = None;
        let mut extra_corner2: Option<Point> = None;

        if i > 0 {
            let p0 = vertices[i - 1];
            let (prev_corner1, prev_corner2, prev_corner3, prev_corner4) =
                compute_corners(p0, p1, stroke_width);
            let cp = cross_product(p0, p1, p2);
            let bevel = should_use_bevel_join(p0, p1, p2, stroke_width, miter_limit);

            if !bevel {
                extra_corner1 = Some(if cp < 0.0 {
                    (
                        (corner1.0 + prev_corner4.0) / 2.0,
                        (corner1.1 + prev_corner4.1) / 2.0,
                    )
                } else {
                    (
                        (corner2.0 + prev_corner3.0) / 2.0,
                        (corner2.1 + prev_corner3.1) / 2.0,
                    )
                });
            }

            if let Some(intersection) = (bevel || cp < 0.0)
                .then(|| get_intersection(prev_corner2, prev_corner3, corner2, corner3))
                .flatten()
            {
                corner2 = intersection;
            }

            if let Some(intersection) = (bevel || cp > 0.0)
                .then(|| get_intersection(prev_corner1, prev_corner4, corner1, corner4))
                .flatten()
            {
                corner1 = intersection;
            }
        }

        if i < vertices.len() - 2 {
            let p3 = vertices[i + 2];
            let (next_corner1, next_corner2, next_corner3, next_corner4) =
                compute_corners(p2, p3, stroke_width);
            let cp = cross_product(p1, p2, p3);
            let bevel = should_use_bevel_join(p1, p2, p3, stroke_width, miter_limit);

            if !bevel {
                extra_corner2 = Some(if cp < 0.0 {
                    (
                        (corner4.0 + next_corner1.0) / 2.0,
                        (corner4.1 + next_corner1.1) / 2.0,
                    )
                } else {
                    (
                        (corner3.0 + next_corner2.0) / 2.0,
                        (corner3.1 + next_corner2.1) / 2.0,
                    )
                });
            }

            if let Some(intersection) = (bevel || cp < 0.0)
                .then(|| get_intersection(next_corner2, next_corner3, corner2, corner3))
                .flatten()
            {
                corner3 = intersection;
            }

            if let Some(intersection) = (bevel || cp > 0.0)
                .then(|| get_intersection(next_corner1, next_corner4, corner1, corner4))
                .flatten()
            {
                corner4 = intersection;
            }
        }

        // Drawing logic
        context.move_to(corner1.0, corner1.1);

        if let Some(ec) = extra_corner1 {
            context.line_to(ec.0, ec.1);
        }

        context.line_to(corner2.0, corner2.1);
        context.line_to(corner3.0, corner3.1);

        if let Some(ec) = extra_corner2 {
            context.line_to(ec.0, ec.1);
        }

        context.line_to(corner4.0, corner4.1);
        context.close_path();

        let mut matrix = Matrix::identity();

        matrix.translate(rect.width() / 2.0 + dist, rect.height() / 2.0);

        dist += ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();

        matrix.rotate((p1.1 - p2.1).atan2(p2.0 - p1.0));

        matrix.translate(-p1.0, -p1.1);

        pattern.set_matrix(matrix);

        pattern.set_extend(cairo::Extend::Repeat);

        context.set_source(&pattern).unwrap();

        context.fill().unwrap();
    }
}
