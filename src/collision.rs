type Item<T> = ((T, T), (T, T));

pub struct Collision<T> {
    items: Vec<Item<T>>,
}

impl <T: PartialOrd> Collision<T> {
    pub fn new() -> Self {
        Collision { items: vec![] }
    }

    pub fn add(&mut self, item: Item<T>) {
        self.items.push(item);
    }

    // pub fn add_xywh(&mut self, x: T, y: T, w: T, h: T) {
    //     self.add(((x, y), (x + w, y + h)));
    // }

    fn overlaps(a: &(T, T), b: &(T, T)) -> bool {
      a.0 <= b.1 && b.0 <= a.1
    }

    pub fn collides(&self, item: Item<T>) -> bool {
        let (x, y) = &item;

        for a in self.items.iter() {
            let (ax, ay) = a;

            if Self::overlaps(x, ax) && Self::overlaps(y, ay) {
                return true;
            }
        }

        false
    }
}
