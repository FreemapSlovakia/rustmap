use crate::size::Size;
use geo::{
    Coord, Geometry, GeometryCollection, Line, LineString, MultiLineString, MultiPoint,
    MultiPolygon, Point, Polygon, Rect, Triangle,
};
use geo_postgis::FromPostgis;

pub struct TileProjector {
    min_x: f64,
    min_y: f64,
    scale_x: f64,
    scale_y: f64,
    height: f64,
}

impl TileProjector {
    pub fn new(bbox: Rect<f64>, size: Size<u32>) -> Self {
        let min = bbox.min();

        Self {
            min_x: min.x,
            min_y: min.y,
            scale_x: size.width as f64 / bbox.width(),
            scale_y: size.height as f64 / bbox.height(),
            height: size.height as f64,
        }
    }

    #[inline]
    pub fn project_coord(&self, coord: &Coord) -> Coord {
        Coord {
            x: (coord.x - self.min_x) * self.scale_x,
            y: self.height - (coord.y - self.min_y) * self.scale_y,
        }
    }
}

pub trait TileProjectable {
    fn project_to_tile(&self, tp: &TileProjector) -> Self;
}

impl TileProjectable for Point {
    fn project_to_tile(&self, tp: &TileProjector) -> Point {
        Point(tp.project_coord(&self.0))
    }
}

impl TileProjectable for LineString {
    fn project_to_tile(&self, tp: &TileProjector) -> LineString {
        LineString::new(self.0.iter().map(|c| tp.project_coord(c)).collect())
    }
}

impl TileProjectable for Line {
    fn project_to_tile(&self, tp: &TileProjector) -> Line {
        Line::new(tp.project_coord(&self.start), tp.project_coord(&self.end))
    }
}

impl TileProjectable for Polygon {
    fn project_to_tile(&self, tp: &TileProjector) -> Polygon {
        Polygon::new(
            self.exterior().project_to_tile(tp),
            self.interiors()
                .iter()
                .map(|ls| ls.project_to_tile(tp))
                .collect(),
        )
    }
}

impl TileProjectable for MultiPoint {
    fn project_to_tile(&self, tp: &TileProjector) -> MultiPoint {
        MultiPoint(self.0.iter().map(|p| p.project_to_tile(tp)).collect())
    }
}

impl TileProjectable for MultiLineString {
    fn project_to_tile(&self, tp: &TileProjector) -> MultiLineString {
        MultiLineString(self.0.iter().map(|ls| ls.project_to_tile(tp)).collect())
    }
}

impl TileProjectable for MultiPolygon {
    fn project_to_tile(&self, tp: &TileProjector) -> MultiPolygon {
        MultiPolygon(self.0.iter().map(|p| p.project_to_tile(tp)).collect())
    }
}

impl TileProjectable for GeometryCollection {
    fn project_to_tile(&self, tp: &TileProjector) -> GeometryCollection {
        GeometryCollection(self.iter().map(|g| g.project_to_tile(tp)).collect())
    }
}

impl TileProjectable for Rect {
    fn project_to_tile(&self, tp: &TileProjector) -> Rect {
        Rect::new(tp.project_coord(&self.min()), tp.project_coord(&self.max()))
    }
}

impl TileProjectable for Triangle {
    fn project_to_tile(&self, tp: &TileProjector) -> Triangle {
        Triangle::new(
            tp.project_coord(&self.0),
            tp.project_coord(&self.1),
            tp.project_coord(&self.2),
        )
    }
}

impl TileProjectable for Geometry {
    fn project_to_tile(&self, tp: &TileProjector) -> Geometry {
        match self {
            Geometry::Point(p) => Geometry::Point(p.project_to_tile(tp)),
            Geometry::Line(l) => Geometry::Line(l.project_to_tile(tp)),
            Geometry::LineString(ls) => Geometry::LineString(ls.project_to_tile(tp)),
            Geometry::Polygon(p) => Geometry::Polygon(p.project_to_tile(tp)),
            Geometry::MultiPoint(mp) => Geometry::MultiPoint(mp.project_to_tile(tp)),
            Geometry::MultiLineString(mls) => Geometry::MultiLineString(mls.project_to_tile(tp)),
            Geometry::MultiPolygon(mp) => Geometry::MultiPolygon(mp.project_to_tile(tp)),
            Geometry::GeometryCollection(gc) => {
                Geometry::GeometryCollection(gc.project_to_tile(tp))
            }
            Geometry::Rect(r) => Geometry::Rect(r.project_to_tile(tp)),
            Geometry::Triangle(t) => Geometry::Triangle(t.project_to_tile(tp)),
        }
    }
}

/////////////////

pub fn geometry_point(row: &postgres::Row) -> Point {
    Point::from_postgis(&row.get::<_, postgis::ewkb::Point>("geometry"))
}

pub fn geometry_line_string(row: &postgres::Row) -> LineString {
    LineString::from_postgis(&row.get::<_, postgis::ewkb::LineString>("geometry"))
}

pub fn geometry_multi_line_string(row: &postgres::Row) -> MultiLineString {
    MultiLineString::from_postgis(&row.get::<_, postgis::ewkb::MultiLineString>("geometry"))
}

pub fn geometry_polygon(row: &postgres::Row) -> Option<Polygon> {
    row.get::<_, Option<postgis::ewkb::Polygon>>("geometry")
        .as_ref()
        .and_then(Option::from_postgis)
}

pub fn geometry_geometry(row: &postgres::Row) -> Option<Geometry> {
    row.get::<_, Option<postgis::ewkb::Geometry>>("geometry")
        .as_ref()
        .and_then(Option::from_postgis)
}
