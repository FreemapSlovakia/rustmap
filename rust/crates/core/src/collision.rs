use crate::colors::ContextExt;
use cairo::Context;
use geo::{CoordNum, Intersects, Rect};
use std::ops::Sub;

const DEBUG: bool = false;

pub struct Collision<'a, T: CoordNum = f64>
where
    T: Sub<Output = T> + PartialOrd + Copy + Into<f64>,
{
    items: Vec<Rect<T>>,
    context: Option<&'a Context>,
}

impl<'a, T: CoordNum> Collision<'a, T>
where
    T: PartialOrd + Sub<Output = T> + Copy + Into<f64>,
{
    pub const fn new(context: Option<&'a Context>) -> Self {
        Self {
            items: vec![],
            context,
        }
    }

    pub fn add(&mut self, item: Rect<T>) -> usize {
        self.items.push(item);

        if DEBUG && let Some(context) = self.context {
            context.rectangle(
                item.min().x.into(),
                item.min().y.into(),
                item.width().into(),
                item.height().into(),
            );

            context.save().expect("context saved");
            context.set_source_color_a((0, 255, 0), 0.5);
            context.set_line_width(1.0);
            context.stroke().unwrap();
            context.restore().expect("context restored");
        }

        self.items.len() - 1
    }

    pub fn collides(&self, bb: &Rect<T>) -> bool {
        let _span = tracy_client::span!("collision::collides");

        let intersects = self.items.iter().any(|item| bb.intersects(item));

        if DEBUG
            && intersects
            && let Some(context) = self.context
        {
            context.rectangle(
                bb.min().x.into(),
                bb.min().y.into(),
                bb.width().into(),
                bb.height().into(),
            );

            context.save().expect("context saved");
            context.set_source_color_a((255, 0, 0), 0.2);
            context.set_line_width(1.0);
            context.stroke().unwrap();
            context.restore().expect("context restored");
        }

        intersects
    }

    pub fn collides_with_exclusion(&self, bbox: &Rect<T>, exclude: usize) -> bool {
        let _span = tracy_client::span!("collision::collides");

        self.items
            .iter()
            .enumerate()
            .any(|(idx, item)| idx != exclude && bbox.intersects(item))
    }
}
