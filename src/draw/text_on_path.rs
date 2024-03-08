use cairo::{Context, Path};
use pango::{AttrInt, AttrList};

pub fn draw_text_on_path(context: &Context, path: &Path, text: &str) {
    let layout = pangocairo::functions::create_layout(context);

    layout.set_text(text);

    let font_desc = pango::FontDescription::from_string("Sans Bold 12");

    layout.set_font_description(Some(&font_desc));

    let attr_list = AttrList::new();

    attr_list.insert(AttrInt::new_letter_spacing(20));

    layout.set_attributes(Some(&attr_list));

    let line = layout.line_readonly(0).unwrap();

    for i in 0..line.length() {
        println!(
            "XXX {} {}",
            line.index_to_x(i, false),
            line.index_to_x(i, true)
        );
    }

    // for run in line.runs() {
    //     let glyph_string = run.glyph_string();

    //     glyph_string.index_to_x(text, analysis, index_, trailing);

    //     for glyph_info in glyph_string.glyph_info() {
    //         let geom = glyph_info.geometry();

    //         // TODO
    //     }
    // }

    // let mut m = offset;
    // let mut px = 0.0;
    // let mut py = 0.0;

    // for ps in path.iter() {
    //     match ps {
    //         PathSegment::MoveTo((x, y)) => {
    //             px = x;
    //             py = y;
    //         }
    //         PathSegment::LineTo((x, y)) => {
    //             let d = (px - x).hypot(py - y);

    //             let mut off = spacing - m;

    //             m += d;

    //             while m >= spacing {
    //                 let t = off / d;
    //                 let xx = px + t * (x - px);
    //                 let yy = py + t * (y - py);

    //                 let angle = (y - py).atan2(x - px);

    //                 // context.move_to(xx, yy);
    //                 // context.arc(xx, yy, 3.0, 0.0, 6.2);
    //                 // context.set_source_rgb(1.0, 0.0, 0.0);
    //                 // context.fill().unwrap();

    //                 draw_maker(xx, yy, angle);

    //                 m -= spacing;
    //                 off += spacing;
    //             }

    //             px = x;
    //             py = y;
    //         }
    //         _ => panic!("invalic path segment type"),
    //     }
    // }
}
