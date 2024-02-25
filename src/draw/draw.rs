use crate::{ctx::Ctx, point::Point};
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use core::slice::Iter;
use postgis::ewkb::{Geometry, Point as PgPoint, Polygon};

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

pub fn draw_geometry(ctx: &Ctx, geom: &Geometry) {
    draw_geometry_uni(geom, &|iter| draw_line(ctx, iter));
}

pub fn draw_geometry_uni<F>(geom: &Geometry, dl: &F)
where
    F: Fn(Iter<PgPoint>) -> (),
{
    match geom {
        Geometry::GeometryCollection(gc) => {
            for geometry in &gc.geometries {
                draw_geometry_uni(geometry, dl);
            }
        }
        Geometry::Polygon(p) => {
            draw_poly(&p, dl);
        }
        Geometry::MultiPolygon(p) => {
            for poly in &p.polygons {
                draw_poly(poly, dl);
            }
        }
        Geometry::MultiLineString(p) => {
            for line in &p.lines {
                dl(line.points.iter());
            }
        }
        Geometry::LineString(p) => {
            dl(p.points.iter());
        }
        _ => {}
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

// pub fn draw_along(ctx: &Ctx, geom: &Geometry) {
//     draw_geometry_uni(geom, &|iter| {
//         for (i, p) in iter.enumerate() {
//             let Point { x, y } = p.project(ctx);

//             if i == 0 {
//                 ctx.context.move_to(x, y);
//             } else {
//                 ctx.context.line_to(x, y);
//             }
//         }
//     });
// }
