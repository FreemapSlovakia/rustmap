use std::ops::Sub;

#[derive(PartialEq, Copy, Clone)]
pub struct BBox<T>
where
    T: Sub<Output = T> + PartialOrd + Copy,
{
    pub min_x: T,
    pub min_y: T,
    pub max_x: T,
    pub max_y: T,
}

impl<T> BBox<T>
where
    T: Sub<Output = T> + PartialOrd + Copy,
{
    pub fn new(min_x: T, min_y: T, max_x: T, max_y: T) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn get_width(&self) -> T {
        self.max_x - self.min_x
    }

    pub fn get_height(&self) -> T {
        self.max_x - self.min_x
    }
}
