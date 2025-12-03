use crate::{bbox::BBox, ctx::Ctx};
use cairo::{Format, ImageSurface};
use gdal::Dataset;

fn read_rgba_from_gdal(
    dataset: &Dataset,
    ctx: &Ctx,
    gt_x_off: f64,
    gt_x_width: f64,
    gt_y_off: f64,
    gt_y_width: f64,
    scale: f64,
) -> ImageSurface {
    let Ctx {
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        size,
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

    let w_scaled = (size.width as f64 * scale) as usize;

    let h_scaled = (size.height as f64 * scale) as usize;

    let band_size = w_scaled * h_scaled;

    let count = dataset.raster_count();

    let mut rgba_data = vec![0u8; band_size * 4];

    let (raster_width, raster_height) = dataset.raster_size();

    // Adjust the window to fit within the raster bounds
    let adj_window_x = window_x.max(0).min(raster_width as isize);
    let adj_window_y = window_y.max(0).min(raster_height as isize);

    let adj_source_width = ((window_x + source_width as isize).min(raster_width as isize)
        - adj_window_x)
        .max(0) as usize;

    let adj_source_height = ((window_y + source_height as isize).min(raster_height as isize)
        - adj_window_y)
        .max(0) as usize;

    let ww = (w_scaled as f64 * (adj_source_width as f64 / source_width as f64)) as usize;
    let hh = (h_scaled as f64 * (adj_source_height as f64 / source_height as f64)) as usize;

    let mut data = vec![0u8; hh * ww];

    for band_index in 0..count {
        let band = dataset.rasterband(band_index + 1).unwrap();

        let no_data = band.no_data_value();

        band.read_into_slice::<u8>(
            (adj_window_x, adj_window_y),
            (adj_source_width, adj_source_height),
            (
                (w_scaled as f64 * (adj_source_width as f64 / source_width as f64)) as usize,
                (h_scaled as f64 * (adj_source_height as f64 / source_height as f64)) as usize,
            ), // Resampled size
            &mut data,
            Some(gdal::raster::ResampleAlg::Lanczos),
        )
        .unwrap();

        for y in 0..w_scaled.min(hh) {
            for x in 0..h_scaled.min(ww) {
                let data_index = y * ww + x;

                let off_y = if window_y == adj_window_y {
                    0
                } else {
                    h_scaled - hh
                };

                let off_x = if window_x == adj_window_x {
                    0
                } else {
                    w_scaled - ww
                };

                let rgba_index = ((y + off_y) * w_scaled + (x + off_x)) * 4;

                let value = data[data_index];

                match (count, band_index) {
                    (1, _) => {
                        rgba_data[rgba_index] = value;
                        rgba_data[rgba_index + 1] = value;
                        rgba_data[rgba_index + 2] = value;
                        rgba_data[rgba_index + 3] = no_data
                            .map_or(255u8, |nd| if (nd as u8) == value { 0u8 } else { 255u8 });
                    }
                    (2, 0) => {
                        rgba_data[rgba_index] = value;
                        rgba_data[rgba_index + 1] = value;
                        rgba_data[rgba_index + 2] = value;
                    }
                    (2, _) => {
                        // alpha
                        rgba_data[rgba_index + 3] = value;
                    }
                    (3, _) => {
                        if band_index == 0 {
                            rgba_data[rgba_index + 3] = 255;
                        }
                        rgba_data[rgba_index + band_index] = value;
                    }
                    (4, _) => {
                        rgba_data[rgba_index + band_index] = value;
                    }
                    _ => panic!("unsupported band count"),
                }
            }
        }
    }

    for i in (0..rgba_data.len()).step_by(4) {
        let alpha = rgba_data[i + 3] as f32 / 255.0;

        let r = (rgba_data[i + 0] as f32 * alpha) as u8;
        let g = (rgba_data[i + 1] as f32 * alpha) as u8;
        let b = (rgba_data[i + 2] as f32 * alpha) as u8;

        rgba_data[i + 0] = b;
        rgba_data[i + 1] = g;
        rgba_data[i + 2] = r;
    }

    let surface = ImageSurface::create_for_data(
        rgba_data.to_vec(),
        Format::ARgb32,
        (size.width as f64 * scale) as i32,
        (size.height as f64 * scale) as i32,
        (size.width as f64 * scale) as i32 * 4,
    )
    .unwrap();

    surface
}

pub fn render(ctx: &Ctx, country: &str) {
    let Ctx {
        context,
        cache,
        zoom,
        scale,
        ..
    } = ctx;

    let cache = &cache.borrow_mut();

    let hillshading_dataset = match cache.hillshading_datasets.get(country) {
        Some(v) => v,
        None => return,
    };

    let [gt_x_off, gt_x_width, _, gt_y_off, _, gt_y_width] =
        hillshading_dataset.geo_transform().unwrap();

    let surface = read_rgba_from_gdal(
        hillshading_dataset,
        ctx,
        gt_x_off,
        gt_x_width,
        gt_y_off,
        gt_y_width,
        *scale,
    );

    context.save().unwrap();

    context.identity_matrix();

    context.set_source_surface(surface, 0.0, 0.0).unwrap();

    context
        .paint_with_alpha(1.0f64.min(1.0 - (*zoom as f64 - 7.0).ln() / 5.0))
        .unwrap();

    context.restore().unwrap();
}
