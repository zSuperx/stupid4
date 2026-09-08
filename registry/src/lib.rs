use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::RwLock,
};

/// A registry that stores items and returns cheap `Id`s to reference them with.
///
/// This struct can only exist in a static context due to the self referential nature of it and its
/// associated `Id`s. Therefore, be ready to leak memory when using this object. It should only be
/// done in environments where a `Registry` must live for the entire program (or in short-lived ones
/// where leaking memory is a non-issue).
///
/// However, it's not all bad! The best part is that the returned `Id`s implement the `Copy` trait
/// :)
#[derive(Debug)]
pub struct Registry<K> {
    map: HashMap<K, usize>,
    vec: Vec<K>,
    lock: RwLock<()>,
}

impl<K> Default for Registry<K> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            vec: Default::default(),
            lock: Default::default(),
        }
    }
}

impl<K: Hash + Clone + Eq> Registry<K> {
    /// Add a value to the store. This will return an `Id` that can look itself up if needed.
    ///
    /// Duplicate values will return the original `Id`. Hence, this can be used as a
    /// "add or get if already exists" function.
    pub fn add(&mut self, value: K) -> Id<K> {
        let _lock = self.lock.write().unwrap();
        let pointer = self as *const Registry<K>;
        if let Some(index) = self.map.get(&value).copied() {
            Id::new(index, pointer)
        } else {
            let index = self.vec.len();
            self.vec.push(value.clone());
            self.map.insert(value, index);
            Id::new(index, pointer)
        }
    }

    /// Returns `Some(Id)` if `value` already exists in the `Registry`.
    /// Returns `None` otherwise.
    pub fn get(&self, value: &K) -> Option<Id<K>> {
        let _lock = self.lock.read().unwrap();
        let pointer = self as *const Registry<K>;
        self.map
            .get(value)
            .copied()
            .map(|index| Id::new(index, pointer))
    }

    /// Looks up the associated value for an `Id` and returns a reference to it.
    pub fn lookup(&self, id: Id<K>) -> &K {
        let _lock = self.lock.read().unwrap();
        &self.vec[id.index]
    }
}

/// A `Copy`able identifier that keeps a reference to the `Registry` that created it.
///
/// Since `Registry`s can (and should) only exist in a `static` context, attempting to use an `Id` after
/// forcibly destroying its parent `Registry` (via `Box::from_raw`) will result in UB.
#[derive(Hash, PartialEq, Eq)]
pub struct Id<K: Hash + Clone + Eq> {
    index: usize,
    pointer: *const Registry<K>,
    _pd: PhantomData<K>,
}

impl<K: Hash + Clone + Eq> Clone for Id<K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: Hash + Clone + Eq> Copy for Id<K> {}

impl<K: Hash + Clone + Eq> Deref for Id<K> {
    type Target = K;

    fn deref(&self) -> &Self::Target {
        self.lookup()
    }
}

impl<K: Hash + Clone + Eq> Id<K> {
    pub fn new(index: usize, pointer: *const Registry<K>) -> Self {
        Self {
            index,
            pointer,
            _pd: PhantomData,
        }
    }

    /// Looks up the associated value for this `Id` and returns a reference to the value it points
    /// to.
    pub fn lookup(&self) -> &K {
        let store: &Registry<K> = unsafe { &*self.pointer };
        store.lookup(*self)
    }
}

impl<K: Hash + Clone + Eq + Debug> Debug for Id<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Id")
            .field("index", &self.index)
            .field("pointer", &self.pointer)
            .field("inner", &self.lookup())
            .finish()
    }
}

impl<K: Hash + Clone + Eq + Display> std::fmt::Display for Id<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.lookup();
        write!(f, "{s}")
    }
}
