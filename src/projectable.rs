use crate::ctx::Ctx;
use geo::Coord;
use postgis::ewkb::Point;

pub trait Projectable {
    fn get(&self) -> Coord;

    fn project(&self, ctx: &Ctx) -> Coord {
        let Ctx { bbox, size, .. } = ctx;

        let coord = self.get();

        Coord {
            x: ((coord.x - bbox.min_x) / bbox.get_width()) * size.width as f64,
            y: (1.0 - ((coord.y - bbox.min_y) / bbox.get_height())) * size.height as f64,
        }
    }
}

impl Projectable for Point {
    fn get(&self) -> Coord {
        Coord {
            x: self.x,
            y: self.y,
        }
    }
}
