use std::fmt::{Debug, Display};

use smallvec::SmallVec;

pub trait InstructionTrait: Display + Debug + Clone {
    type Val;

    fn is_terminator(&self) -> bool;
    fn defs(&self) -> Box<dyn Iterator<Item = &Self::Val> + '_>;
    fn uses(&self) -> Box<dyn Iterator<Item = &Self::Val> + '_>;

    fn values(&self) -> impl Iterator<Item = &Self::Val> {
        self.defs().into_iter().chain(self.uses().into_iter())
    }
}
