use crate::point::Point;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};

pub fn offset_line(iter: impl IntoIterator<Item = Point>, offset: f64) -> Vec<Point> {
    let mut result = Vec::<Point>::new();

    let mut polyline = Polyline::new();

    for p in iter {
        polyline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
    }

    for pc in polyline.parallel_offset(offset) {
        for v in pc.arcs_to_approx_lines(1.0).unwrap().vertex_data {
            result.push(Point::new(v.x, v.y));
        }
    }

    result
}
