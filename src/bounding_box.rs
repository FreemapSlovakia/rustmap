pub struct BoundingBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl BoundingBox {
    // Constructor for creating a new BoundingBox instance
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        BoundingBox {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    // Method to expand the bounding box to include another bounding box
    pub fn expand(&mut self, other: &BoundingBox) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
    }

    // Method to extend the bounding box to include a point (x, y)
    pub fn extend_by_point(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }
}


impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }
}
