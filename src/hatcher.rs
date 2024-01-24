use crate::{ctx::Ctx, draw::Projectable};
use lyon_algorithms::{
    geom::{euclid::Point2D, Angle},
    hatching::{HatchSegment, Hatcher, HatchingOptions, RegularHatchingPattern},
    path::Path,
};
use postgis::ewkb::Point;
use std::slice::Iter;

pub fn hatch(ctx: &Ctx, iter: Iter<Point>) {
    let mut op = Path::builder();

    for (i, p) in iter.enumerate() {
        let (x, y) = p.project(ctx);

        let point = Point2D::new(x as f32, y as f32);

        if i == 0 {
            op.begin(point);
        } else {
            op.line_to(point);
        }
    }

    op.end(true);

    let context = &ctx.context;

    let mut hatcher = Hatcher::new();

    let ho = HatchingOptions::DEFAULT.with_angle(Angle::degrees(45.0));

    // ho.uv_origin = Point2D::new(10.0, 2.0);

    hatcher.hatch_path(
        op.build().iter(),
        &ho,
        &mut RegularHatchingPattern {
            interval: 10.0,
            callback: &mut |segment: &HatchSegment| {
                context.move_to(segment.a.position.x as f64, segment.a.position.y as f64);
                context.line_to(segment.b.position.x as f64, segment.b.position.y as f64);
                context.stroke().unwrap();
            },
        },
    );
}
