use std::rc::Rc;
use std::{collections::HashMap, fmt::Debug, hash::Hash};

#[derive(Debug)]
pub struct Pool<T: Eq + Hash + Clone + Debug> {
    map: HashMap<Rc<T>, usize>,
    vec: Vec<Rc<T>>,
}

impl<T: Eq + Hash + Clone + Debug> Default for Pool<T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            vec: Default::default(),
        }
    }
}

impl<T: Eq + Hash + Clone + Debug> Pool<T> {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
            vec: Default::default(),
        }
    }

    pub fn get(&mut self, item: &T) -> Rc<T> {
        let index = match self.map.get(item) {
            Some(index) => *index,
            None => {
                let index = self.vec.len();
                let arc = Rc::new(item.clone());
                self.map.insert(arc.clone(), index);
                self.vec.push(arc);
                index
            }
        };
        self.vec[index].clone()
    }
}
