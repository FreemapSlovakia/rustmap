use crate::{ctx::Ctx, draw::draw::Projectable, point::Point};
use core::slice::Iter;
use swash::{scale::ScaleContext, shape::ShapeContext, text::Script, zeno::Verb, FontRef};

struct Segment {
    points: Vec<Point>,
}

impl Segment {
    fn average_position(&self) -> Point {
        if self.points.len() < 2 {
            return self.points[0];
        }

        let mut total_x = 0.0;
        let mut total_y = 0.0;
        let mut total_length = 0.0;

        for i in 0..self.points.len() - 1 {
            let p1 = self.points[i];
            let p2 = self.points[i + 1];
            let mid_point = p1.interpolate(&p2, 0.5);
            let segment_length = p1.distance_to(&p2);

            total_x += mid_point.x * segment_length;
            total_y += mid_point.y * segment_length;
            total_length += segment_length;
        }

        if total_length > 0.0 {
            Point {
                x: total_x / total_length,
                y: total_y / total_length,
            }
        } else {
            Point::new(0.0, 0.0)
        }
    }

    fn average_normal(&self) -> Point {
        if self.points.len() < 2 {
            return Point::new(0.0, 0.0);
        }

        let mut total_normal = Point::new(0.0, 0.0);
        let mut total_length = 0.0;

        for i in 0..self.points.len() - 1 {
            let p1 = self.points[i];
            let p2 = self.points[i + 1];
            let edge_vector = Point {
                x: p2.x - p1.x,
                y: p2.y - p1.y,
            };
            let edge_length = p1.distance_to(&p2);
            let edge_normal = Point {
                x: -edge_vector.y,
                y: edge_vector.x,
            };

            total_normal.x += edge_normal.x * edge_length;
            total_normal.y += edge_normal.y * edge_length;
            total_length += edge_length;
        }

        if total_length > 0.0 {
            Point {
                x: total_normal.x / total_length,
                y: total_normal.y / total_length,
            }
        } else {
            Point::new(0.0, 0.0)
        }
    }
}

pub fn text_on_line(ctx: &Ctx, iter: Iter<postgis::ewkb::Point>, text: &str) {
    let pts: Vec<Point> = iter.map(|p| p.project(ctx)).rev().collect();

    let font_data = std::fs::read("/home/martin/.fonts/NotoSans-Regular.ttf").unwrap();

    let font = FontRef::from_index(&font_data, 0).unwrap();

    let mut context = ShapeContext::new();

    let mut shaper = context
        .builder(font)
        .script(Script::Latin)
        .size(16.)
        .build();

    shaper.add_str(text);

    let mut context = ScaleContext::new();

    let mut scaler = context.builder(font).size(16.).hint(true).build();

    let cr = &ctx.context;

    let mut current_index = 0;
    let mut accumulated_length = 0.0;
    let mut segment_start_point = pts[0];

    shaper.shape_with(|gc| {
        for glyph in gc.glyphs {
            let outline = scaler.scale_outline(glyph.id).unwrap();

            let bounds = outline.bounds();

            cr.rectangle(
                bounds.min.x as f64,
                bounds.min.y as f64,
                bounds.width() as f64,
                bounds.height() as f64,
            );

            let length = bounds.width() as f64;

            let mut segment_points = vec![segment_start_point];
            let mut current_length = 0.0;

            while current_index < pts.len() - 1 && current_length < length {
                let p1 = pts[current_index];

                let p2 = pts[current_index + 1];

                let segment_length = p1.distance_to(&p2);

                accumulated_length += segment_length;

                if accumulated_length > length {
                    let excess = accumulated_length - length;

                    let t = (segment_length - excess) / segment_length;

                    let split_point = p1.interpolate(&p2, t);

                    segment_points.push(split_point);

                    accumulated_length -= excess; // Reset for the next segment

                    current_length += segment_length - excess;

                    segment_start_point = split_point; // Start next segment from here

                    break;
                } else {
                    segment_points.push(p2);

                    current_length += segment_length;

                    current_index += 1;
                }
            }

            let segment = Segment {
                points: segment_points,
            };

            let avg_position = segment.average_position();

            let avg_normal = segment.average_normal();

            cr.translate(avg_position.x, avg_position.y);

            // cr.set_source_rgb(1.0, 0.0, 0.0);

            // cr.set_line_width(0.1);
            // cr.stroke().unwrap();

            let points = outline.points();

            let mut i = 0;

            for verb in outline.verbs().into_iter() {
                match verb {
                    Verb::MoveTo => {
                        cr.move_to(points[i].x as f64, points[i].y as f64);

                        i += 1;
                    }
                    Verb::LineTo => {
                        cr.line_to(points[i].x as f64, points[i].y as f64);

                        i += 1;
                    }
                    Verb::CurveTo => {
                        cr.curve_to(
                            points[i].x as f64,
                            points[i].y as f64,
                            points[i + 1].x as f64,
                            points[i + 1].y as f64,
                            points[i + 2].x as f64,
                            points[i + 2].y as f64,
                        );

                        i += 3;
                    }
                    Verb::QuadTo => {
                        let current_point = cr.current_point().unwrap_or((0.0, 0.0));

                        let control = points[i];

                        let point = points[i + 1];

                        cr.curve_to(
                            current_point.0
                                + (control.x as f64 - current_point.0) * 2.0 / 3.0 as f64,
                            current_point.1
                                + (control.y as f64 - current_point.1) * 2.0 / 3.0 as f64,
                            point.x as f64 + (control.x - point.x) as f64 * 2.0 / 3.0 as f64,
                            point.y as f64 + (control.y - point.y) as f64 * 2.0 / 3.0 as f64,
                            point.x as f64,
                            point.y as f64,
                        );

                        i += 2;
                    }
                    Verb::Close => {
                        cr.close_path();
                    }
                }
            }

            // cr.set_source_rgb(0.0, 0.0, 1.0);

            cr.fill().unwrap();

            // cr.fill_extents()

            // cr.translate(glyph.advance as f64, 0.0);
        }
    });
}
