use crate::ctx::Ctx;
use cairo::{Format, ImageSurface};
use gdal::Dataset;

fn read_rgba_from_gdal(
    dataset: &Dataset,
    ctx: &Ctx,
    gt_x_off: f64,
    gt_x_width: f64,
    gt_y_off: f64,
    gt_y_width: f64,
) -> Vec<u8> {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        size: (w, h),
        ..
    } = ctx;

    // Convert geographic coordinates (min_x, min_y, max_x, max_y) to pixel coordinates
    let pixel_min_x = ((min_x - gt_x_off) / gt_x_width).round() as isize;
    let pixel_max_x = ((max_x - gt_x_off) / gt_x_width).round() as isize;
    let pixel_max_y = ((min_y - gt_y_off) / gt_y_width).round() as isize;
    let pixel_min_y = ((max_y - gt_y_off) / gt_y_width).round() as isize;

    let window_x = pixel_min_x;
    let window_y = pixel_min_y;
    let source_width = (pixel_max_x - pixel_min_x) as usize;
    let source_height = (pixel_max_y - pixel_min_y) as usize;

    let off = (w * h) as usize;

    let mut data = vec![0u8; off];

    let mut rgba_data = vec![0u8; off * 4];

    for band_index in 0..4 as usize {
        let band = dataset.rasterband(band_index as isize + 1).unwrap();

        band.read_into_slice::<u8>(
            (window_x, window_y),
            (source_width, source_height),
            (*w as usize, *h as usize), // Resampled size
            &mut data,
            Some(gdal::raster::ResampleAlg::Lanczos),
        )
        .unwrap();

        for i in 0..off {
            rgba_data[i * 4 + band_index] = data[i];
        }
    }

    // TODO get rid of this step
    for i in (0..rgba_data.len()).step_by(4) {
        let alpha = rgba_data[i + 3] as f32 / 255.0;

        let r = (rgba_data[i + 0] as f32 * alpha) as u8;
        let g = (rgba_data[i + 1] as f32 * alpha) as u8;
        let b = (rgba_data[i + 2] as f32 * alpha) as u8;

        rgba_data[i + 0] = b;
        rgba_data[i + 1] = g;
        rgba_data[i + 2] = r;
    }

    rgba_data
}

pub fn render(ctx: &Ctx, zoom: u32) {
    let Ctx {
        context,
        size: (w, h),
        ..
    } = ctx;

    let hillshading_dataset = Dataset::open("/home/martin/14TB/hillshading/sk/final.tif").unwrap();

    let [gt_x_off, gt_x_width, _, gt_y_off, _, gt_y_width] =
        hillshading_dataset.geo_transform().unwrap();

    let rgba_data = read_rgba_from_gdal(
        &hillshading_dataset,
        ctx,
        gt_x_off,
        gt_x_width,
        gt_y_off,
        gt_y_width,
    );

    let surface = ImageSurface::create_for_data(
        rgba_data.to_vec(),
        Format::ARgb32,
        *w as i32,
        *h as i32,
        *w as i32 * 4,
    )
    .unwrap();

    context.set_source_surface(surface, 0.0, 0.0).unwrap();

    context.paint_with_alpha(1.0f64.min(1.0 - (zoom as f64 - 7.0).ln() / 5.0)).unwrap();
}