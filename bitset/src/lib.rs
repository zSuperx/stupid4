/// A set specialized for unsigned integers.
///
/// This is implemented internally as a `Vec<u128>`, and uses bitwise operations in order to
/// implement set functions like union, difference, and intersect.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct BitSet(Vec<u128>);

impl BitSet {
    /// Creates an empty `BitSet`
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Creates a `BitSet` from the contents of an iterator over `usize`s
    ///
    /// This does nothing fancy at the moment, it literally just runs a for loop to insert all
    /// items.
    pub fn from_iterator(items: impl Iterator<Item = usize>) -> Self {
        let mut s = Self::new();
        for i in items {
            s.insert(i);
        }
        s
    }

    /// Creates an empty `BitSet` with an underlying size of `size`. This can still grow if a large
    /// enough index is inserted, but can help prevent reallocations when inserting many numbers in a
    /// loop.
    pub fn with_size(size: usize) -> Self {
        Self(vec![0; (size / 128) + 1])
    }

    /// Inserts an index into the set and returns whether that index was newly inserted.
    ///
    /// If the given index is larger than the capcity of the map, the map is resized to fit it.
    ///
    /// This means that repeatedly inserting larger and larger values will result in repeated
    /// reallocations. If the max size of the set is known before-hand, it may be desirable to use
    /// [`BitSet::with_size`] instead, which pre-allocates the underlying `Vec`.
    pub fn insert(&mut self, index: usize) -> bool {
        let bucket = index / 128;
        let subindex = index % 128;
        if bucket + 1 > self.0.len() {
            self.0.resize(bucket + 1, 0);
        }
        let ret = self.0[bucket] & (1 << subindex) != 0;
        self.0[bucket] |= 1 << subindex;
        !ret
    }

    /// Removes the given index from the set and returns whether an item was actually removed.
    ///
    /// If the index is larger than the capacity of the map, the map is NOT resized and `false` is
    /// returned.
    pub fn remove(&mut self, index: usize) -> bool {
        let bucket = index / 128;
        let subindex = index % 128;
        if bucket + 1 > self.0.len() {
            return false;
        }
        self.0[bucket] &= !(1 << subindex);
        true
    }

    /// Checks if the set contains the given index.
    ///
    /// If the index is larger than the capacity of the map, the map is NOT resized and `false` is
    /// returned.
    pub fn contains(&self, index: usize) -> bool {
        let bucket = index / 128;
        let subindex = index % 128;
        if bucket + 1 > self.0.len() {
            return false;
        }
        self.0[bucket] & (1 << subindex) != 0
    }

    /// Perform set union on `self` and `other`, producing a new `BitSet`.
    ///
    /// *This does not modify `self` in-place.*
    pub fn union(&self, other: &Self) -> Self {
        let max = self.0.len().max(other.0.len());
        Self(
            (0..max)
                .map(|i| self.0.get(i).unwrap_or(&0) | other.0.get(i).unwrap_or(&0))
                .collect(),
        )
    }

    /// Perform set union on `self` and `other`, modifying `self` in place
    pub fn union_eq(&mut self, other: &Self) {
        let max = self.0.len().max(other.0.len());
        self.0.resize(max, 0);
        (0..max).for_each(|i| self.0[i] |= other.0.get(i).unwrap_or(&0));
    }

    /// Perform set difference on `self` and `other`, producing a new `BitSet`.
    ///
    /// *This does not modify `self` in-place.*
    pub fn difference(&self, other: &Self) -> Self {
        let max = self.0.len().max(other.0.len());
        Self(
            (0..max)
                .map(|i| self.0.get(i).unwrap_or(&0) & !other.0.get(i).unwrap_or(&0))
                .collect(),
        )
    }

    /// Perform set intersection on `self` and `other`, producing a new `BitSet`.
    ///
    /// *This does not modify `self` in-place.*
    pub fn intersect(&self, other: &Self) -> Self {
        let max = self.0.len().max(other.0.len());
        Self(
            (0..max)
                .map(|i| self.0.get(i).unwrap_or(&0) & other.0.get(i).unwrap_or(&0))
                .collect(),
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> {
        let mut vec = vec![];
        for (index, bucket) in self.0.iter().enumerate() {
            for subindex in 0..128 {
                if (*bucket & (1 << subindex)) != 0 {
                    vec.push(subindex + index * 128);
                }
            }
        }
        vec.into_iter()
    }
}

impl std::ops::BitAnd for BitSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersect(&rhs)
    }
}

impl std::ops::BitOr for BitSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(&rhs)
    }
}

impl std::ops::BitOrAssign for BitSet {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(&rhs);
    }
}

impl std::ops::BitAndAssign for BitSet {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersect(&rhs);
    }
}

impl std::ops::Sub for BitSet {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.difference(&rhs)
    }
}

impl std::fmt::Debug for BitSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for bucket in self.0.iter().rev() {
            f.write_fmt(format_args!("{bucket:0128b}"))?;
        }
        Ok(())
    }
}

impl std::fmt::Display for BitSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        f.write_fmt(format_args!("{{{s}}}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print() {
        let mut bs = BitSet::new();
        bs.insert(0);
        bs.insert(5);

        bs.remove(5);
        assert_eq!(bs.0[0], 1)
    }

    #[test]
    fn test_union() {
        let mut a = BitSet::new();
        a.insert(1);
        a.insert(4);
        a.insert(5);
        let mut b = BitSet::new();
        b.insert(2);
        b.insert(3);
        let u = a.union(&b);
        println!("{a}");
        println!("{b}");
        println!("{u}");

        assert_eq!(u.0[0], 0b111110)
    }

    #[test]
    fn test_difference() {
        let mut a = BitSet::new();
        a.insert(1);
        a.insert(2);
        a.insert(3);
        a.insert(4);
        a.insert(5);

        let mut b = BitSet::new();
        b.insert(2);
        b.insert(3);

        let d = a.difference(&b);
        println!("{a}");
        println!("{b}");
        println!("{d}");

        assert_eq!(d.0[0], 0b110010)
    }

    #[test]
    fn test_intersect() {
        let mut a = BitSet::new();
        a.insert(1);
        a.insert(2);
        a.insert(3);
        a.insert(4);
        a.insert(5);

        let mut b = BitSet::new();
        b.insert(2);
        b.insert(3);

        let i = a.intersect(&b);
        println!("{a}");
        println!("{b}");
        println!("{i}");

        assert_eq!(i.0[0], 0b001100)
    }

    #[test]
    fn test_from_iter() {
        let a = BitSet::from_iterator(0..5);
        assert_eq!(a.0[0], 0b11111)
    }
}
