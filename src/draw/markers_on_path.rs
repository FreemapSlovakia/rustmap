use cairo::{Path, PathSegment};

pub fn draw_markers_on_path<F>(path: &Path, offset: f64, spacing: f64, draw_maker: &F)
where
    F: Fn(f64, f64, f64) -> (),
{
    let mut m = offset;
    let mut px = 0.0;
    let mut py = 0.0;

    for ps in path.iter() {
        match ps {
            PathSegment::MoveTo((x, y)) => {
                px = x;
                py = y;
            }
            PathSegment::LineTo((x, y)) => {
                let d = (px - x).hypot(py - y);

                let mut off = spacing - m;

                m += d;

                while m >= spacing {
                    let t = off / d;
                    let xx = px + t * (x - px);
                    let yy = py + t * (y - py);

                    let angle = (y - py).atan2(x - px);

                    // context.move_to(xx, yy);
                    // context.arc(xx, yy, 3.0, 0.0, 6.2);
                    // context.set_source_rgb(1.0, 0.0, 0.0);
                    // context.fill().unwrap();

                    draw_maker(xx, yy, angle);

                    m -= spacing;
                    off += spacing;
                }

                px = x;
                py = y;
            }
            _ => panic!("invalic path segment type"),
        }
    }
}
