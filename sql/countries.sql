-- Country polygons, used to pick per-country rendering rules (see MAPRENDER_PLACE_TYPE_OVERRIDES).
--
-- Built from the admin_level=2 boundary relations imported with borders.yaml, so it needs
-- both tables from that import: osm_country_members (the border ways) and
-- osm_country_relations (their ISO 3166-1 codes). Re-run it after re-importing borders.
--
-- The polygons are subdivided because the renderer does a point-in-polygon lookup per
-- place label; whole-country polygons would make that lookup slow.

BEGIN;

DROP TABLE IF EXISTS countries;

CREATE TABLE countries AS
WITH rings AS (
  -- Node the ways so that ones merely touching at their endpoints close into rings.
  -- The dimension filter drops the label / admin_centre node members.
  SELECT osm_id, role = 'inner' AS is_inner, ST_Node(ST_Collect(geometry)) AS lines
  FROM osm_country_members
  WHERE ST_Dimension(geometry) = 1
  GROUP BY osm_id, role = 'inner'
),
faces AS (
  SELECT osm_id, is_inner, (ST_Dump(ST_Polygonize(ARRAY[lines]))).geom AS geom
  FROM rings
),
-- Enclaves (Lesotho, San Marino, Vatican, …) are inner rings of the surrounding country.
enclaves AS (
  SELECT osm_id, ST_Union(geom) AS geom
  FROM faces
  WHERE is_inner
  GROUP BY osm_id
),
areas AS (
  SELECT f.osm_id, CASE WHEN e.geom IS NULL THEN f.geom ELSE ST_Difference(f.geom, e.geom) END AS geom
  FROM faces f LEFT JOIN enclaves e USING (osm_id)
  WHERE NOT f.is_inner
)
SELECT
  lower(r.iso3166_1) AS country,
  ST_Subdivide(a.geom, 255) AS geometry
FROM areas a
  -- imposm stores relation ids negated in member tables but not everywhere, hence abs().
  JOIN osm_country_relations r ON abs(r.osm_id) = abs(a.osm_id)
WHERE COALESCE(r.iso3166_1, '') <> '' AND NOT ST_IsEmpty(a.geom);

CREATE INDEX countries_geometry_gix
  ON countries
  USING GIST (geometry);

ANALYZE countries;

COMMIT;
