use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
pub struct Scopes<K: Hash + Eq, V: Clone> {
    scopes: Vec<HashMap<K, V>>,
}

impl<K: Hash + Eq, V: Clone> Default for Scopes<K, V> {
    fn default() -> Self {
        Self {
            scopes: vec![HashMap::default()],
        }
    }
}

impl<K: Hash + Eq, V: Clone> Scopes<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_base(map: HashMap<K, V>) -> Self {
        Self { scopes: vec![map] }
    }

    pub fn insert(&mut self, name: K, val: V) -> Option<V> {
        self.scopes.last_mut().unwrap().insert(name, val)
    }

    pub fn get(&self, name: &K) -> Option<V> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn push_filled_scope(&mut self, map: HashMap<K, V>) {
        self.scopes.push(map);
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
}
