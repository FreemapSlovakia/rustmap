extern crate cairo;
extern crate pangocairo;

use cairo::{Context, Format, ImageSurface, Path, PathSegment, RecordingSurface};
use pangocairo::{
    functions::{create_layout, layout_line_path},
    pango::FontDescription,
};
use std::fs::File;

use crate::point::Point;

fn fancy_cairo_stroke(cr: &Context) {
    _fancy_cairo_stroke(cr, false);
}

fn fancy_cairo_stroke_preserve(cr: &Context) {
    _fancy_cairo_stroke(cr, true);
}

fn _fancy_cairo_stroke(cr: &Context, preserve: bool) {
    cr.save().unwrap();
    cr.set_source_rgb(1.0, 0.0, 0.0);

    let line_width = cr.line_width();
    let path = cr.copy_path().unwrap();
    cr.new_path();

    cr.save().unwrap();
    cr.set_line_width(line_width / 3.0);
    cr.set_dash(&[10.0, 10.0], 0.0);

    for data in path.iter() {
        match data {
            PathSegment::MoveTo((x, y)) => {
                cr.move_to(x, y);
            }
            PathSegment::LineTo((x, y)) => {
                cr.move_to(x, y);
            }
            PathSegment::CurveTo((x1, y1), (x2, y2), (x3, y3)) => {
                cr.line_to(x1, y1);
                cr.move_to(x2, y2);
                cr.line_to(x3, y3);
            }
            PathSegment::ClosePath => {}
        }
    }

    cr.stroke().unwrap();
    cr.restore().unwrap();

    cr.save().unwrap();
    cr.set_line_width(line_width * 4.0);
    cr.set_line_cap(cairo::LineCap::Round);

    for data in path.iter() {
        match data {
            PathSegment::MoveTo((x, y)) => {
                cr.move_to(x, y);
            }
            PathSegment::LineTo((x, y)) => {
                cr.rel_line_to(0.0, 0.0);
                cr.move_to(x, y);
            }
            PathSegment::CurveTo((x1, y1), (x2, y2), (x3, y3)) => {
                cr.rel_line_to(0.0, 0.0);
                cr.move_to(x1, y1);
                cr.rel_line_to(0.0, 0.0);
                cr.move_to(x2, y2);
                cr.rel_line_to(0.0, 0.0);
                cr.move_to(x3, y3);
            }
            PathSegment::ClosePath => {
                cr.rel_line_to(0.0, 0.0);
            }
        }
    }

    cr.rel_line_to(0.0, 0.0);
    cr.stroke().unwrap();
    cr.restore().unwrap();

    for data in path.iter() {
        match data {
            PathSegment::MoveTo((x, y)) => {
                cr.move_to(x, y);
            }
            PathSegment::LineTo((x, y)) => {
                cr.line_to(x, y);
            }
            PathSegment::CurveTo((x1, y1), (x2, y2), (x3, y3)) => {
                cr.curve_to(x1, y1, x2, y2, x3, y3);
            }
            PathSegment::ClosePath => {
                cr.close_path();
            }
        }
    }

    cr.stroke().unwrap();

    if preserve {
        cr.append_path(&path);
    }

    cr.restore().unwrap();
}

fn curve_length(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> f64 {
    let surface = RecordingSurface::create(cairo::Content::Color, None).unwrap();

    let cr = Context::new(&surface).unwrap();

    cr.move_to(x0, y0);

    cr.curve_to(x1, y1, x2, y2, x3, y3);

    let mut length = 0.0;

    let mut curr_x = Default::default();

    let mut curr_y = Default::default();

    let path = cr.copy_path_flat().unwrap();

    for data in path.iter() {
        match data {
            PathSegment::MoveTo((x, y)) => {
                curr_x = x;
                curr_y = y;
            }
            PathSegment::LineTo((x, y)) => {
                length += (x - curr_x).hypot(y - curr_y);
            }
            _ => {
                panic!("unexpected path segment");
            }
        }
    }

    length
}

type Parametrization = f64;

fn parametrize_path(path: &Path) -> Vec<Parametrization> {
    let mut parametrization = Vec::with_capacity(path.iter().count());

    let mut current_point = Point::default();
    let mut last_move_to = Point::default();

    for data in path.iter() {
        match data {
            PathSegment::MoveTo((x, y)) => {
                parametrization.push(0.0);
                last_move_to = Point::new(x, y);
                current_point = Point::new(x, y);
            }
            PathSegment::LineTo((x, y)) => {
                parametrization.push((current_point.x - x).hypot(current_point.y - y));
                current_point = Point::new(x, y);
            }
            PathSegment::CurveTo((x1, y1), (x2, y2), (x3, y3)) => {
                parametrization.push(curve_length(
                    current_point.x,
                    current_point.y,
                    x1,
                    y1,
                    x2,
                    y2,
                    x3,
                    y3,
                ));

                current_point = Point::new(x3, y3);
            }
            PathSegment::ClosePath => {
                parametrization.push(
                    (current_point.x - last_move_to.x).hypot(current_point.y - last_move_to.y),
                );

                current_point = last_move_to;
            }
        }
    }

    parametrization
}

type TransformPointFunc = fn(&ParametrizedPath, &mut f64, &mut f64);

fn transform_path(cr: &Context, path: &Path, f: TransformPointFunc, closure: &ParametrizedPath) {
    for data in path.iter() {
        match data {
            PathSegment::MoveTo((mut x, mut y)) => {
                f(closure, &mut x, &mut y);

                cr.move_to(x, y);
            }
            PathSegment::LineTo((mut x, mut y)) => {
                f(closure, &mut x, &mut y);
                cr.line_to(x, y);
            }
            PathSegment::CurveTo((mut x1, mut y1), (mut x2, mut y2), (mut x3, mut y3)) => {
                f(closure, &mut x3, &mut y3);
                f(closure, &mut x2, &mut y2);
                f(closure, &mut x1, &mut y1);

                cr.curve_to(x1, y1, x2, y2, x3, y3);
            }
            PathSegment::ClosePath => {
                cr.close_path();
            }
        }
    }
}

struct ParametrizedPath<'a> {
    path: &'a cairo::Path,
    parametrization: Vec<Parametrization>,
}

fn point_on_path(param: &ParametrizedPath, x: &mut f64, y: &mut f64) {
    let mut the_x = *x;
    let the_y = *y;
    let path = param.path;
    let parametrization = &param.parametrization;
    let mut current_point = Point::default();
    let mut last_move_to = Point::default();

    for (i, data) in path.iter().enumerate() {
        if the_x <= parametrization[i]
            && match data {
                PathSegment::MoveTo(_) => false,
                _ => true,
            }
        {
            let mut line_to = |x1: f64, y1: f64| {
                let ratio = the_x / parametrization[i];
                *x = current_point.x * (1.0 - ratio) + x1 * ratio;
                *y = current_point.y * (1.0 - ratio) + y1 * ratio;
                let dx = -(current_point.x - x1);
                let dy = -(current_point.y - y1);
                let ratio = the_y / dx.hypot(dy);
                *x += -dy * ratio;
                *y += dx * ratio;
            };

            match data {
                PathSegment::MoveTo(_) => {}
                PathSegment::LineTo((x, y)) => {
                    line_to(x, y);
                }
                PathSegment::CurveTo((x1, y1), (x2, y2), (x3, y3)) => {
                    /* FIXME the formulas here are not exactly what we want, because the
                     * Bezier parametrization is not uniform.  But I don't know how to do
                     * better.  The caller can do slightly better though, by flattening the
                     * Bezier and avoiding this branch completely.  That has its own cost
                     * though, as large y values magnify the flattening error drastically.
                     */

                    let ratio = the_x / parametrization[i];
                    let ratio_1_0 = ratio;
                    let ratio_0_1 = 1.0 - ratio;
                    let ratio_2_0 = ratio_1_0 * ratio_1_0;
                    let ratio_0_2 = ratio_0_1 * ratio_0_1;
                    let ratio_3_0 = ratio_2_0 * ratio_1_0;
                    let ratio_2_1 = ratio_2_0 * ratio_0_1;
                    let ratio_1_2 = ratio_1_0 * ratio_0_2;
                    let ratio_0_3 = ratio_0_1 * ratio_0_2;
                    let _1_4ratio_1_0_3ratio_2_0 = 1.0 - 4.0 * ratio_1_0 + 3.0 * ratio_2_0;
                    let _2ratio_1_0_3ratio_2_0 = 2.0 * ratio_1_0 - 3.0 * ratio_2_0;
                    *x = current_point.x * ratio_0_3
                        + 3.0 * x1 * ratio_1_2
                        + 3.0 * x2 * ratio_2_1
                        + x3 * ratio_3_0;
                    *y = current_point.y * ratio_0_3
                        + 3.0 * y1 * ratio_1_2
                        + 3.0 * y2 * ratio_2_1
                        + y3 * ratio_3_0;
                    let dx = -3.0 * current_point.x * ratio_0_2
                        + 3.0 * x1 * _1_4ratio_1_0_3ratio_2_0
                        + 3.0 * x2 * _2ratio_1_0_3ratio_2_0
                        + 3.0 * x3 * ratio_2_0;
                    let dy = -3.0 * current_point.y * ratio_0_2
                        + 3.0 * y1 * _1_4ratio_1_0_3ratio_2_0
                        + 3.0 * y2 * _2ratio_1_0_3ratio_2_0
                        + 3.0 * y3 * ratio_2_0;
                    let ratio = the_y / (dx * dx + dy * dy).sqrt();
                    *x += -dy * ratio;
                    *y += dx * ratio;
                }
                PathSegment::ClosePath => {
                    line_to(last_move_to.x, last_move_to.y);
                }
            }

            break;
        }

        the_x -= parametrization[i];

        match data {
            PathSegment::MoveTo((x, y)) => {
                current_point = Point::new(x, y);
                last_move_to = Point::new(x, y);
            }
            PathSegment::LineTo((x, y)) => {
                current_point = Point::new(x, y);
            }
            PathSegment::CurveTo(_, _, (x3, y3)) => {
                current_point = Point::new(x3, y3);
            }
            PathSegment::ClosePath => {}
        }
    }
}

fn map_path_onto(cr: &Context, path: &cairo::Path) {
    let current_path = cr.copy_path().unwrap();

    cr.new_path();

    transform_path(
        cr,
        &current_path,
        point_on_path,
        &ParametrizedPath {
            path: path,
            parametrization: parametrize_path(path),
        },
    );

    // cr.append_path(&current_path);
}

fn draw_text(cr: &Context, x: f64, y: f64, font: &str, text: &str) {
    let layout = create_layout(cr);
    let desc = FontDescription::from_string(font);
    layout.set_font_description(Some(&desc));
    layout.set_text(text);
    let line = layout.line(0).unwrap();
    cr.move_to(x, y);
    layout_line_path(cr, &line);
}

fn draw_twisted(cr: &Context, x: f64, y: f64, font: &str, text: &str) {
    cr.save().unwrap();
    cr.set_tolerance(0.01);
    let path = cr.copy_path_flat().unwrap();
    cr.new_path();
    draw_text(cr, x, y, font, text);
    map_path_onto(cr, &path);
    cr.fill_preserve().unwrap();
    cr.save().unwrap();
    cr.set_source_rgb(0.1, 0.1, 0.1);
    cr.stroke().unwrap();
    cr.restore().unwrap();
    cr.restore().unwrap();
}

fn draw_dream(cr: &Context) {
    cr.move_to(50.0, 650.0);
    cr.rel_line_to(250.0, 50.0);
    cr.rel_curve_to(250.0, 50.0, 600.0, -50.0, 600.0, -250.0);
    cr.rel_curve_to(0.0, -400.0, -300.0, -100.0, -800.0, -300.0);
    cr.set_line_width(1.5);
    cr.set_source_rgba(0.3, 0.3, 1.0, 0.3);
    fancy_cairo_stroke_preserve(cr);
    draw_twisted(
        cr,
        0.0,
        0.0,
        "Serif 72",
        "It was a dream... Oh Just a dream...",
    );
}

fn draw_wow(cr: &Context) {
    cr.move_to(400.0, 780.0);
    cr.rel_curve_to(50.0, -50.0, 150.0, -50.0, 200.0, 0.0);
    cr.scale(1.0, 2.0);
    cr.set_line_width(2.0);
    cr.set_source_rgba(0.3, 1.0, 0.3, 1.0);
    fancy_cairo_stroke_preserve(cr);
    draw_twisted(cr, -20.0, -150.0, "Serif 60", "WOW!");
}

pub fn main() {
    let surface = ImageSurface::create(Format::ARgb32, 1000, 800).unwrap();
    let cr = Context::new(&surface).unwrap();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().unwrap();
    draw_dream(&cr);
    draw_wow(&cr);

    let mut file = File::create("/home/martin/x.png").expect("Couldn't create file.");

    surface.write_to_png(&mut file).unwrap();
}
