CREATE OR REPLACE FUNCTION make_buffered_envelope(
    bbox_xmin FLOAT,
    bbox_ymin FLOAT,
    bbox_xmax FLOAT,
    bbox_ymax FLOAT,
    zoom INTEGER,
    buffer_pixels INTEGER
)
RETURNS geometry AS $$
DECLARE
    meters_per_pixel FLOAT;
    buffer_distance FLOAT;
BEGIN
    -- Calculate meters per pixel at the given zoom level
    meters_per_pixel := 40075016.686 / (256 * pow(2, zoom));

    -- Convert the buffer from pixels to meters
    buffer_distance := buffer_pixels * meters_per_pixel;

    -- Create the geometry with buffer
    RETURN ST_MakeEnvelope(
        bbox_xmin - buffer_distance,
        bbox_ymin - buffer_distance,
        bbox_xmax + buffer_distance,
        bbox_ymax + buffer_distance,
        3857
    );
END;
$$ LANGUAGE plpgsql;
