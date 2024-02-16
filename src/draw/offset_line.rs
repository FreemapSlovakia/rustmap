use crate::{ctx::Ctx, point::Point};
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use core::slice::Iter;
use postgis::ewkb::Point as PgPoint;

use super::draw::Projectable;

pub fn offset_line(ctx: &Ctx, iter: Iter<PgPoint>, offset: f64) -> Vec<Point> {
  let mut result = Vec::<Point>::new();

  let mut polyline = Polyline::new();

  for p in iter {
      let Point { x, y } = p.project(ctx);

      polyline.add_vertex(PlineVertex::new(x, y, 0.0));
  }

  for pc in polyline.parallel_offset(offset) {
      for v in pc.arcs_to_approx_lines(1.0).unwrap().vertex_data {
          result.push(Point::new(v.x, v.y));
      }
  }

  result
}
