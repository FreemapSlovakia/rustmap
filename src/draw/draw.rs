use cairo::Context;
use cavalier_contours::polyline::{PlineSource, PlineSourceMut, PlineVertex, Polyline};
use geo::{Geometry, LineString, Polygon};

pub fn draw_geometry(context: &Context, geom: &Geometry) {
    draw_geometry_uni(geom, &|iter| draw_line(context, iter));
}

pub fn draw_geometry_uni<F>(geom: &Geometry, dl: &F)
where
    F: Fn(&LineString),
{
    match geom {
        Geometry::GeometryCollection(gc) => {
            for geometry in gc {
                draw_geometry_uni(geometry, dl);
            }
        }
        Geometry::Polygon(p) => {
            draw_poly(p, dl);
        }
        Geometry::MultiPolygon(mp) => {
            for p in mp {
                draw_poly(p, dl);
            }
        }
        Geometry::MultiLineString(mls) => {
            for ls in mls {
                dl(ls);
            }
        }
        Geometry::LineString(ls) => {
            dl(ls);
        }
        _ => {}
    }
}

fn draw_poly<F>(poly: &Polygon, dl: &F)
where
    F: Fn(&LineString),
{
    dl(poly.exterior());

    for ring in poly.interiors() {
        dl(ring);
    }
}

pub fn draw_line(context: &Context, line_string: &LineString) {
    for (i, p) in line_string.into_iter().enumerate() {
        if i == 0 {
            context.move_to(p.x, p.y);
        } else {
            context.line_to(p.x, p.y);
        }
    }
}

pub fn draw_line_off(context: &Context, line_string: &LineString, offset: f64) {
    let mut polyline = Polyline::new();

    for p in line_string {
        polyline.add_vertex(PlineVertex::new(p.x, p.y, 0.0));
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
