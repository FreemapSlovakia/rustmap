use crate::{ctx::Ctx, layers::hillshading_datasets::HillshadingDatasets};
use cairo::{Format, ImageSurface};
use gdal::Dataset;

fn read_rgba_from_gdal(
    dataset: &Dataset,
    ctx: &Ctx,
    gt_x_off: f64,
    gt_x_width: f64,
    gt_y_off: f64,
    gt_y_width: f64,
    raster_scale: f64,
) -> (ImageSurface, bool) {
    let bbox = ctx.bbox;
    let size = ctx.size;

    let min = bbox.min();
    let max = bbox.max();

    // Convert geographic coordinates (min_x, min_y, max_x, max_y) to pixel coordinates
    let pixel_min_x = ((min.x - gt_x_off) / gt_x_width).floor() as isize;
    let pixel_max_x = ((max.x - gt_x_off) / gt_x_width).ceil() as isize;

    let pixel_y0 = (min.y - gt_y_off) / gt_y_width;
    let pixel_y1 = (max.y - gt_y_off) / gt_y_width;
    let pixel_min_y = pixel_y0.min(pixel_y1).floor() as isize;
    let pixel_max_y = pixel_y0.max(pixel_y1).ceil() as isize;

    let window_x = pixel_min_x;
    let window_y = pixel_min_y;
    let source_width = (pixel_max_x - pixel_min_x) as usize;
    let source_height = (pixel_max_y - pixel_min_y) as usize;

    let w_scaled = (size.width as f64 * raster_scale) as usize;

    let h_scaled = (size.height as f64 * raster_scale) as usize;

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

    let ww = (w_scaled as f64 * (adj_source_width as f64 / source_width as f64)).ceil() as usize;
    let hh = (h_scaled as f64 * (adj_source_height as f64 / source_height as f64)).ceil() as usize;

    let offset_x = (((adj_window_x - window_x) as f64 / source_width as f64) * w_scaled as f64)
        .floor()
        .max(0.0) as usize;

    let offset_y = (((adj_window_y - window_y) as f64 / source_height as f64) * h_scaled as f64)
        .floor()
        .max(0.0) as usize;

    let copy_w = ww.min(w_scaled.saturating_sub(offset_x));
    let copy_h = hh.min(h_scaled.saturating_sub(offset_y));

    let mut data = vec![0u8; hh * ww];

    let mut used_data = false;

    for band_index in 0..count {
        let band = dataset.rasterband(band_index + 1).unwrap();

        let no_data = band.no_data_value();

        band.read_into_slice::<u8>(
            (adj_window_x, adj_window_y),
            (adj_source_width, adj_source_height),
            (ww, hh), // Resampled size
            &mut data,
            Some(gdal::raster::ResampleAlg::Lanczos),
        )
        .unwrap();

        for y in 0..copy_h {
            for x in 0..copy_w {
                let data_index = y * ww + x;

                let rgba_index = ((y + offset_y) * w_scaled + (x + offset_x)) * 4;

                let value = data[data_index];
                let is_no_data = no_data.map_or(false, |nd| (nd as u8) == value);

                if !is_no_data {
                    used_data = true;
                }

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

        let r = (rgba_data[i] as f32 * alpha) as u8;
        let g = (rgba_data[i + 1] as f32 * alpha) as u8;
        let b = (rgba_data[i + 2] as f32 * alpha) as u8;

        rgba_data[i] = b;
        rgba_data[i + 1] = g;
        rgba_data[i + 2] = r;
    }

    let surface = ImageSurface::create_for_data(
        rgba_data.to_vec(),
        Format::ARgb32,
        (size.width as f64 * raster_scale) as i32,
        (size.height as f64 * raster_scale) as i32,
        (size.width as f64 * raster_scale) as i32 * 4,
    )
    .unwrap();

    (surface, used_data)
}

pub fn render(
    ctx: &Ctx,
    country: &str,
    alpha: f64,
    shading_data: &mut HillshadingDatasets,
    raster_scale: f64,
) {
    let (surface, used_data) = {
        let hillshading_dataset = shading_data
            .get(country)
            .unwrap_or_else(|| panic!("no such dataset {country}"));

        let [gt_x_off, gt_x_width, _, gt_y_off, _, gt_y_width] =
            hillshading_dataset.geo_transform().unwrap();

        read_rgba_from_gdal(
            hillshading_dataset,
            ctx,
            gt_x_off,
            gt_x_width,
            gt_y_off,
            gt_y_width,
            raster_scale,
        )
    };

    if used_data {
        shading_data.record_use(country);
    }

    let context = ctx.context;

    context.save().expect("context saved");

    context.identity_matrix();
    if raster_scale != 1.0 {
        context.scale(1.0 / raster_scale, 1.0 / raster_scale);
    }

    context.set_source_surface(surface, 0.0, 0.0).unwrap();

    context.paint_with_alpha(alpha).unwrap();

    context.restore().expect("context restored");
}
