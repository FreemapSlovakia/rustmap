use cairo::Context;

use crate::{bbox::BBox, colors::ContextExt};
use std::ops::Sub;

pub struct Collision<'a, T>
where
    T: Sub<Output = T> + PartialOrd + Copy + Into<f64>,
{
    items: Vec<BBox<T>>,
    context: Option<&'a Context>,
}

impl<'a, T> Collision<'a, T>
where
    T: PartialOrd + Sub<Output = T> + Copy + Into<f64>,
{
    pub const fn new(context: Option<&'a Context>) -> Self {
        Self {
            items: vec![],
            context,
        }
    }

    pub fn add(&mut self, item: BBox<T>) -> usize {
        self.items.push(item);

        if let Some(context) = self.context {
            context.rectangle(
                item.min_x.into(),
                item.min_y.into(),
                item.get_width().into(),
                item.get_height().into(),
            );

            context.save().expect("context saved");
            context.set_source_color_a((255, 0, 0), 0.5);
            context.set_line_width(1.0);
            context.stroke().unwrap();
            context.restore().expect("context restored");
        }

        self.items.len() - 1
    }

    fn overlaps(min_a: T, max_a: T, min_b: T, max_b: T) -> bool {
        min_a <= max_b && min_b <= max_a
    }

    pub fn collides(&self, bb: &BBox<T>) -> bool {
        let _span = tracy_client::span!("collision::collides");

        self.items.iter().any(|item| {
            Self::overlaps(bb.min_x, bb.max_x, item.min_x, item.max_x)
                && Self::overlaps(bb.min_y, bb.max_y, item.min_y, item.max_y)
        })
    }

    pub fn collides_with_exclusion(&self, bbox: &BBox<T>, exclude: usize) -> bool {
        let _span = tracy_client::span!("collision::collides");

        self.items.iter().enumerate().any(|(idx, item)| {
            idx != exclude
                && Self::overlaps(bbox.min_x, bbox.max_x, item.min_x, item.max_x)
                && Self::overlaps(bbox.min_y, bbox.max_y, item.min_y, item.max_y)
        })
    }
}
