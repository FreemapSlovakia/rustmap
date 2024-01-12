// use cairo::{Content, Context, RecordingSurface, Rectangle};
use gdal::Dataset;
// use rsvg::Loader;
// use std::collections::HashMap;

pub struct Cache {
    pub hillshading_dataset: Dataset,
    // pub svg_map: HashMap<&'a str, &'a RecordingSurface>,
}

// impl<'a> Cache<'a> {
//     fn get_svg(path: &str) {
//         let handle = Loader::new().read_path(path).unwrap();

//         let renderer = rsvg::CairoRenderer::new(&handle);

//         let dim = renderer.intrinsic_size_in_pixels().unwrap();

//         let tile = RecordingSurface::create(
//             Content::ColorAlpha,
//             Some(Rectangle::new(0.0, 0.0, dim.0, dim.1)),
//         )
//         .unwrap();

//         {
//             let context = Context::new(&tile).unwrap();

//             renderer
//                 .render_document(&context, &cairo::Rectangle::new(0.0, 0.0, dim.0, dim.1))
//                 .unwrap();
//         }
//     }
// }
