use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use geo::{Coord, LineString};

pub fn offset_line(line_string: &LineString, offset: f64) -> LineString {
    let mut result = Vec::<Coord>::new();

    let mut polyline = Polyline::new();

    for p in line_string {
        polyline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
    }

    for pc in polyline.parallel_offset(offset) {
        for v in pc.arcs_to_approx_lines(1.0).unwrap().vertex_data {
            result.push(Coord { x: v.x, y: v.y });
        }
    }

    LineString(result)
}
