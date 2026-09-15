//! This module defines the [`BasicBlock`] struct, which represents a bundle of instructions
//! terminated by a control-flow-inducing instruction.
use crate::common::Label;
use std::collections::BTreeSet;

use bitset::BitSet;

use super::traits::InstructionTrait;

#[derive(Debug, Clone)]
pub struct BasicBlock<I: InstructionTrait> {
    pub label: Label<I>,
    pub successors: BTreeSet<Label<I>>,
    pub predecessors: BTreeSet<Label<I>>,
    pub fallthrough: Option<Label<I>>,
    pub instructions: Vec<I>,

    pub live_in: BitSet,
    pub live_out: BitSet,
    pub gen_: BitSet,
    pub kill: BitSet,
}

impl<I: InstructionTrait> BasicBlock<I> {
    pub fn new(label: Label<I>) -> Self {
        Self {
            label,
            successors: Default::default(),
            predecessors: Default::default(),
            fallthrough: Default::default(),
            instructions: Default::default(),
            live_in: Default::default(),
            live_out: Default::default(),
            gen_: Default::default(),
            kill: Default::default(),
        }
    }

    pub fn terminator(&self) -> Option<&I> {
        self.instructions.last().filter(|i| i.is_terminator())
    }

    pub fn delete_terminator(&mut self) -> Option<I> {
        self.instructions.pop_if(|i| i.is_terminator())
    }

    /// This function calls the provided `rewriter` closure on each instruction within the
    /// `BasicBlock`. The closure returns a [`RewriteAction`], which this function uses to determine
    /// how to proceed with the rewrite.
    pub fn rewrite(&mut self, mut rewriter: impl FnMut(&I) -> RewriteAction<I>) {
        let old = std::mem::take(&mut self.instructions);
        for i in old {
            match rewriter(&i) {
                RewriteAction::Skip => continue,
                RewriteAction::Keep => self.instructions.push(i),
                RewriteAction::Replace(items) => self.instructions.extend(items),
                RewriteAction::InsertBefore(items) => {
                    self.instructions.extend(items);
                    self.instructions.push(i);
                }
                RewriteAction::InsertAfter(items) => {
                    self.instructions.push(i);
                    self.instructions.extend(items);
                }
            }
        }
    }
}

pub enum RewriteAction<I: InstructionTrait> {
    Skip,
    Keep,
    Replace(Vec<I>),
    InsertBefore(Vec<I>),
    InsertAfter(Vec<I>),
}
