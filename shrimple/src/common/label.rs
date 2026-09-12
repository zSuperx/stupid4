use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Label<I>(
    pub(crate) &'static str,
    pub(crate) usize,
    pub(crate) PhantomData<I>,
);

impl<I> Label<I> {
    pub fn name(&self) -> &'static str {
        self.0
    }
}

impl<I> PartialOrd for Label<I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.0.partial_cmp(&other.0) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.1.partial_cmp(&other.1) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.2.partial_cmp(&other.2)
    }
}

impl<I> Ord for Label<I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.1.cmp(&other.1)
    }
}

impl<I> Copy for Label<I> {}
impl<I> Clone for Label<I> {
    fn clone(&self) -> Self {
        Self(self.0, self.1, self.2)
    }
}

impl<I> std::hash::Hash for Label<I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state);
        self.2.hash(state);
    }
}

impl<I> Eq for Label<I> {}
impl<I> PartialEq for Label<I> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1 && self.2 == other.2
    }
}

impl<I> std::fmt::Display for Label<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Label(name, index, _) = self;
        if name.is_empty() {
            f.write_fmt(format_args!(".{index}"))
        } else {
            f.write_fmt(format_args!(".{name}.{index}"))
        }
    }
}
