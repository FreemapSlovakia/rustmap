const EARTH_RADIUS: f64 = 6_378_137.0; // Equatorial radius of the Earth in meters (WGS 84)

const HALF_CIRCUMFERENCE: f64 = std::f64::consts::PI * EARTH_RADIUS;

pub fn tile_bounds_to_epsg3857(x: u32, y: u32, z: u32, tile_size: u32) -> (f64, f64, f64, f64) {
    let total_pixels = tile_size as f64 * 2f64.powf(z as f64);
    let pixel_size = (2.0 * HALF_CIRCUMFERENCE) / total_pixels;

    let min_x = x as f64 * tile_size as f64 * pixel_size - HALF_CIRCUMFERENCE;
    let max_y = HALF_CIRCUMFERENCE - y as f64 * tile_size as f64 * pixel_size;

    let max_x = min_x + tile_size as f64 * pixel_size;
    let min_y = max_y - tile_size as f64 * pixel_size;

    (min_x, min_y, max_x, max_y)
}

pub fn bbox_size_in_pixels(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    zoom: f64,
) -> (u32, u32) {
    let width_meters = max_x - min_x;
    let height_meters = max_y - min_y;

    let total_span_meters = 2.0 * HALF_CIRCUMFERENCE;
    let resolution = total_span_meters / (256.0 * 2f64.powf(zoom as f64));

    let width_pixels = (width_meters / resolution).round() as u32;
    let height_pixels = (height_meters / resolution).round() as u32;

    (width_pixels, height_pixels)
}

pub fn to_absolute_pixel_coords(x: f64, y: f64, zoom: u8) -> (f64, f64) {
    // Tile size in pixels (usually 256 or 512)
    let tile_size: f64 = 256.0;

    // Earth's radius in the same units as x, y (meters for EPSG:3857)

    // Total number of tiles in one row or column at the given zoom level
    let num_tiles = 2f64.powi(zoom as i32);

    // Total map circumference at this zoom level
    let total_map_circumference = num_tiles * tile_size;

    // Convert x, y to pixel coordinates
    let pixel_x = (x + HALF_CIRCUMFERENCE) / (2.0 * HALF_CIRCUMFERENCE) * total_map_circumference;
    let pixel_y = (HALF_CIRCUMFERENCE - y) / (2.0 * HALF_CIRCUMFERENCE) * total_map_circumference;

    (pixel_x, pixel_y)
}
 // x /  2 ^ zoom * 256
