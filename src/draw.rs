use crate::{bounding_box::BoundingBox, ctx::Ctx, point::Point};
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use core::slice::Iter;
use postgis::ewkb::{Geometry, LineString, Point as PgPoint, Polygon};

impl BoundingBox {
    pub fn extend_by_polygon(&mut self, polygon: &Polygon) {
        for ring in polygon.rings.iter() {
            for point in ring.points.iter() {
                self.extend_by_point(point.x, point.y);
            }
        }
    }

    pub fn extend_by_line_string(&mut self, line_string: &LineString) {
        for point in line_string.points.iter() {
            self.extend_by_point(point.x, point.y);
        }
    }

    pub fn extend_by_geometry(&mut self, geometry: &Geometry) {
        match geometry {
            Geometry::MultiPolygon(multipolygon) => {
                for polygon in multipolygon.polygons.iter() {
                    self.extend_by_polygon(polygon);
                }
            }
            Geometry::Polygon(polygon) => {
                self.extend_by_polygon(polygon);
            }
            Geometry::MultiLineString(multi_line_string) => {
                for line_string in multi_line_string.lines.iter() {
                    self.extend_by_line_string(line_string);
                }
            }
            Geometry::LineString(line_string) => {
                self.extend_by_line_string(line_string);
            }
            Geometry::Point(point) => {
                self.extend_by_point(point.x, point.y);
            }
            Geometry::MultiPoint(multi_point) => {
                for point in multi_point.points.iter() {
                    self.extend_by_point(point.x, point.y);
                }
            }
            Geometry::GeometryCollection(gc) => {
                for geom in gc.geometries.iter() {
                    self.extend_by_geometry(geom);
                }
            }
        }
    }
}

pub fn draw_mpoly(ctx: &Ctx, geom: &Geometry) {
    draw_mpoly_uni(geom, |iter| draw_line(ctx, iter));
}

pub fn draw_mpoly_uni<F>(geom: &Geometry, dl: F)
where
    F: Fn(Iter<PgPoint>) -> (),
{
    match geom {
        Geometry::Polygon(p) => {
            draw_poly(&p, &dl);
        }
        Geometry::MultiPolygon(p) => {
            for poly in &p.polygons {
                draw_poly(poly, &dl);
            }
        }
        _ => {
            panic!("not a polygon");
        }
    }
}

fn draw_poly<F>(poly: &Polygon, dl: &F)
where
    F: Fn(Iter<PgPoint>) -> (),
{
    for ring in &poly.rings {
        dl(ring.points.iter());
    }
}

pub fn draw_line(ctx: &Ctx, iter: Iter<PgPoint>) {
    for (i, p) in iter.enumerate() {
        let Point { x, y } = p.project(ctx);

        if i == 0 {
            ctx.context.move_to(x, y);
        } else {
            ctx.context.line_to(x, y);
        }
    }
}

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

pub fn draw_line_off(ctx: &Ctx, iter: Iter<PgPoint>, offset: f64) {
    let mut polyline = Polyline::new();

    let context = &ctx.context;

    for p in iter {
        let Point { x, y } = p.project(ctx);

        polyline.add_vertex(PlineVertex::new(x, y, 0.0));
    }

    for pc in polyline.parallel_offset(offset) {
        let mut first = true;
        let mut p1 = (0.0, 0.0);
        let mut prev_bulge = 0.0;

        for v in pc.vertex_data {
            if first {
                context.move_to(v.x, v.y);
                first = false;
                p1 = (v.x, v.y);
                prev_bulge = v.bulge;
            } else {
                let p2 = (v.x, v.y);

                if prev_bulge == 0.0 {
                    context.line_to(p2.0, p2.1);
                } else {
                    let theta = 4.0 * prev_bulge.atan();
                    let dist = ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
                    let radius = dist / (2.0 * (theta / 2.0).sin());

                    // Calculate center of the arc
                    let mx = (p1.0 + p2.0) / 2.0;
                    let my = (p1.1 + p2.1) / 2.0;
                    let l = (radius.powi(2) - (dist / 2.0).powi(2)).sqrt();
                    let direction = if prev_bulge > 0.0 { 1.0 } else { -1.0 };
                    let ox = mx - direction * l * (p2.1 - p1.1) / dist;
                    let oy = my + direction * l * (p2.0 - p1.0) / dist;

                    // Calculate start and end angles
                    let mut start_angle = (p1.1 - oy).atan2(p1.0 - ox) + 5.0 * std::f64::consts::PI;
                    let mut end_angle = (p2.1 - oy).atan2(p2.0 - ox) + 5.0 * std::f64::consts::PI;

                    if prev_bulge > 0.0 {
                        start_angle += std::f64::consts::PI;
                        end_angle += std::f64::consts::PI;

                        while end_angle < start_angle {
                            end_angle += 2.0 * std::f64::consts::PI;
                        }

                        let mut angle = start_angle;

                        while angle < end_angle {
                            angle += std::f64::consts::PI / 10.0;

                            context.line_to(ox + radius * angle.cos(), oy + radius * angle.sin());

                            break;
                        }
                    } else {
                        while end_angle > start_angle {
                            end_angle -= 2.0 * std::f64::consts::PI;
                        }

                        let mut angle = start_angle;

                        while angle > end_angle {
                            angle -= std::f64::consts::PI / 10.0;

                            context.line_to(ox + radius * angle.cos(), oy + radius * angle.sin());
                        }
                    }
                    // if prev_bulge > 0.0 {
                    //     context.arc(ox, oy, radius.abs(), start_angle, end_angle);
                    // } else {
                    //     context.arc_negative(ox, oy, radius.abs(), start_angle, end_angle);
                    // }
                }

                p1 = p2;
                prev_bulge = v.bulge;
            }
        }
    }
}

pub trait Projectable {
    fn get(&self) -> (f64, f64);

    fn project(&self, ctx: &Ctx) -> Point {
        let Ctx {
            bbox: (min_x, min_y, max_x, max_y),
            size: (w_i, h_i),
            ..
        } = ctx;

        let w = *w_i as f64;
        let h = *h_i as f64;

        let (px, py) = self.get();

        let x = ((px - min_x) / (max_x - min_x)) * w;

        let y = h - ((py - min_y) / (max_y - min_y)) * h;

        Point::new(x, y)
    }
}

impl Projectable for PgPoint {
    fn get(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

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
