BEGIN;

-- ============================================================
-- Z5–Z7  (150 m simplify, 512 vertices)
-- ============================================================

DROP TABLE IF EXISTS land_z5_7;

CREATE TABLE land_z5_7 AS
SELECT
  ST_Subdivide(
    ST_SimplifyPreserveTopology(geom, 150),
    512
  ) AS geometry
FROM land_polygons_raw
WHERE geom IS NOT NULL;

CREATE INDEX land_z5_7_geometry_gix
  ON land_z5_7
  USING GIST (geometry);

ANALYZE land_z5_7;



-- ============================================================
-- Z8–Z10  (30 m simplify, 512 vertices)
-- ============================================================

DROP TABLE IF EXISTS land_z8_10;

CREATE TABLE land_z8_10 AS
SELECT
  ST_Subdivide(
    ST_SimplifyPreserveTopology(geom, 30),
    512
  ) AS geometry
FROM land_polygons_raw
WHERE geom IS NOT NULL;

CREATE INDEX land_z8_10_geometry_gix
  ON land_z8_10
  USING GIST (geometry);

ANALYZE land_z8_10;



-- ============================================================
-- Z11–Z13  (4 m simplify, 512 vertices)
-- ============================================================

DROP TABLE IF EXISTS land_z11_13;

CREATE TABLE land_z11_13 AS
SELECT
  ST_Subdivide(
    ST_SimplifyPreserveTopology(geom, 4),
    512
  ) AS geometry
FROM land_polygons_raw
WHERE geom IS NOT NULL;

CREATE INDEX land_z11_13_geometry_gix
  ON land_z11_13
  USING GIST (geometry);

ANALYZE land_z11_13;



-- ============================================================
-- Z14+  (NO simplification, 512 vertices)
-- ============================================================

DROP TABLE IF EXISTS land_z14_plus;

CREATE TABLE land_z14_plus AS
SELECT
  ST_Subdivide(
    geom,
    512
  ) AS geometry
FROM land_polygons_raw
WHERE geom IS NOT NULL;

CREATE INDEX land_z14_plus_geometry_gix
  ON land_z14_plus
  USING GIST (geometry);

ANALYZE land_z14_plus;



COMMIT;
