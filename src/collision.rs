use crate::bbox::BBox;
use std::ops::Sub;

pub struct Collision<T>
where
    T: Sub<Output = T> + PartialOrd + Copy,
{
    items: Vec<BBox<T>>,
}

impl<T> Collision<T>
where
    T: PartialOrd + Sub<Output = T> + Copy,
{
    pub const fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn add(&mut self, item: BBox<T>) {
        self.items.push(item);
    }

    fn overlaps(min_a: T, max_a: T, min_b: T, max_b: T) -> bool {
        min_a <= max_b && min_b <= max_a
    }

    pub fn collides(&self, item: BBox<T>) -> bool {
        for BBox {
            min_x,
            min_y,
            max_x,
            max_y,
        } in &self.items
        {
            if Self::overlaps(item.min_x, item.max_x, *min_x, *max_x)
                && Self::overlaps(item.min_y, item.max_y, *min_y, *max_y)
            {
                return true;
            }
        }

        false
    }
}
