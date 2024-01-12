use cairo::Context;
use rsvg::Loader;

pub fn render(context: &Context) {
    let handle = Loader::new().read_path("aerodrome.svg").unwrap();

    let renderer = rsvg::CairoRenderer::new(&handle);

    let dim = renderer.intrinsic_size_in_pixels().unwrap();

    renderer
        .render_document(&context, &cairo::Rectangle::new(20.0, 20.0, dim.0, dim.1))
        .unwrap();
}
