//! This module defines the generic [`FunctionBuilder`] object
//!
//! A `FunctionBuilder` is generic over 3 types:
//! 1. Instruction
//! 2. Value
//! 3. Type
//!
//! Targets should define type aliases to create target-specific `FunctionBuilder`s
//!
//! For example, the STIR ISA defines its `IRFunction` as follows:
//!
//! `pub type IRFunction = FunctionBuilder<IRInstr, IRValue, IRType>;`
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::marker::PhantomData;
use std::rc::Rc;

use registry::{Id, Registry};

use crate::stir::isa::IRType;

use crate::common::{BasicBlock, InstructionTrait, Label};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDef {
    pub(crate) size: usize,
    pub(crate) alignment: usize,
    pub(crate) fields: Vec<IRType>,
}

pub type StructId = Id<StructDef>;

#[derive(Default, Debug)]
pub struct ModuleBuilder<I: InstructionTrait, V, T> {
    pub(crate) functions: BTreeMap<String, FunctionBuilder<I, V, T>>,
    pub(crate) reg_count: usize,
    pub(crate) block_count: usize,
    pub(crate) structs: Rc<RefCell<Registry<StructDef>>>,
}

impl<I: InstructionTrait, V, T> ModuleBuilder<I, V, T> {
    pub fn new() -> Self {
        Self {
            functions: Default::default(),
            reg_count: 0,
            block_count: 0,
            structs: Default::default(),
        }
    }
    pub fn createFunction(
        &mut self,
        name: String,
        return_type: T,
    ) -> &mut FunctionBuilder<I, V, T> {
        let f = FunctionBuilder::new(name.clone(), return_type);
        let None = self.functions.insert(name.clone(), f) else {
            panic!("Duplicate function");
        };
        self.functions.get_mut(&name).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionBuilder<I: InstructionTrait, V, T> {
    pub(crate) name: String,
    pub(crate) cursor: Label<I>,
    pub(crate) args: Vec<(V, T)>,
    pub(crate) return_type: T,
    pub(crate) entrypoint: Label<I>,
    pub(crate) reg_count: usize,
    pub(crate) block_count: usize,
    pub(crate) blocks: BTreeMap<Label<I>, BasicBlock<I>>,
}

/// Given an `IRBuilder<I>` and format args, expands to `$builder.emit(Comment(format!(...)))`
///
/// This relies on the existence of an `I::Comment(String)` instruction for the instruction type of
/// the `IRBuilder`.
#[macro_export]
macro_rules! comment {
    ($builder:expr, $($fmtargs:tt)*) => {
        $builder.emit(Comment(format!($($fmtargs)*)))
    }
}

impl<I: InstructionTrait, V, T> FunctionBuilder<I, V, T> {
    /// Creates a new IR Function builder. The function is initialized with an empty BasicBlock as
    /// its entrypoint.
    ///
    /// The insert point is set to this entrypoint, so you can start emitting immediately after
    /// creating it.
    pub fn new(name: String, return_type: T) -> Self {
        let cursor = Label("entrypoint", 0, PhantomData::default());
        let blocks = BTreeMap::from([(cursor, BasicBlock::empty())]);
        let block_count = 1;
        let reg_count = 0;

        Self {
            cursor,
            name,
            return_type,
            reg_count,
            block_count,
            blocks,
            entrypoint: cursor,
            args: Default::default(),
        }
    }

    pub fn addArg(&mut self, arg_value: V, arg_type: T) {
        self.args.push((arg_value, arg_type));
    }

    pub fn getRegCount(&self) -> usize {
        self.reg_count
    }

    pub fn setRegCount(&mut self, new: usize) {
        self.reg_count = new;
    }

    /// Performs a visitor pass that verifies no reachable block in the function lacks a terminator.
    ///
    /// Upon finding an invalid block, the verifier either breaks with `false`, OR if a
    /// `default_return` instruction is provided, will set the block's terminator to that.
    pub fn verify(&mut self, default_return: Option<I>) -> bool {
        !self.dfs_short_circuit(|builder, curr_id| {
            let block = builder.blocks.get_mut(&curr_id).unwrap();
            if block.terminator.is_none() {
                match default_return.as_ref() {
                    Some(i) => block.terminator = Some(i.clone()),
                    None => return true,
                }
            }
            false
        })
    }

    pub fn setInsertPoint(&mut self, block: Label<I>) {
        self.cursor = block;
    }

    pub fn getInsertPoint(&mut self) -> Label<I> {
        self.cursor
    }

    pub fn getReturnType(&self) -> &T {
        &self.return_type
    }

    pub fn newBlock(&mut self) -> Label<I> {
        let id = Label("", self.block_count, Default::default());
        self.block_count += 1;
        self.blocks.insert(id, BasicBlock::empty());
        id
    }

    pub fn newNamedBlock(&mut self, block_name: &'static str) -> Label<I> {
        let id = Label(block_name, self.block_count, Default::default());
        self.block_count += 1;
        self.blocks.insert(id, BasicBlock::new(block_name));
        id
    }

    /// Retrieves the entrypoint of the function
    pub fn getEntryPoint(&self) -> Label<I> {
        self.entrypoint
    }

    /// Replaces a function's entrypoint, returning the old one
    pub fn setEntryPoint(&mut self, new_entrypoint: Label<I>) -> Label<I> {
        let ret = self.entrypoint;
        self.entrypoint = new_entrypoint;
        ret
    }

    #[inline]
    pub fn removeFallthrough(&mut self) -> Option<Label<I>> {
        self.removeFallthroughFrom(self.cursor)
    }

    pub fn removeFallthroughFrom(&mut self, this: Label<I>) -> Option<Label<I>> {
        let this_block = self.blocks.get_mut(&this).unwrap();
        this_block.fallthrough.take()
    }

    #[inline]
    pub fn addFallthrough(&mut self, fallthrough: Label<I>) {
        self.addFallthroughTo(self.cursor, fallthrough);
    }

    pub fn addFallthroughTo(&mut self, this: Label<I>, fallthrough: Label<I>) {
        let this_block = self.blocks.get_mut(&this).unwrap();
        this_block.fallthrough = Some(fallthrough);
        this_block.successors.insert(fallthrough);

        let other_block = self.blocks.get_mut(&fallthrough).unwrap();
        other_block.predecessors.insert(this);
    }

    #[inline]
    pub fn addSuccessors(&mut self, successors: &[Label<I>]) {
        self.addSuccessorsTo(self.cursor, successors);
    }

    pub fn addSuccessorsTo(&mut self, this: Label<I>, successors: &[Label<I>]) {
        for succ_id in successors {
            let this_block = self.blocks.get_mut(&this).unwrap();
            this_block.successors.insert(*succ_id);

            let other_block = self.blocks.get_mut(succ_id).unwrap();
            other_block.predecessors.insert(this);
        }
    }

    #[inline]
    pub fn addPredecessors(&mut self, successors: &[Label<I>]) {
        self.addPredecessorsTo(self.cursor, successors);
    }

    pub fn addPredecessorsTo(&mut self, this: Label<I>, predecessors: &[Label<I>]) {
        for pred_id in predecessors {
            assert_eq!(this.0, pred_id.0);
            let this_block = self.blocks.get_mut(&this).unwrap();
            this_block.predecessors.insert(*pred_id);

            let other_block = self.blocks.get_mut(pred_id).unwrap();
            other_block.successors.insert(this);
        }
    }

    pub fn emit(&mut self, instr: I) {
        let basic_block = self.blocks.get_mut(&self.cursor).unwrap();
        if basic_block.terminator.is_some() {
            return;
        }
        if instr.is_terminator() {
            basic_block.terminator = Some(instr);
        } else {
            basic_block.instructions.push(instr);
        }
    }

    pub fn isCurrentTerminated(&self) -> bool {
        self.blocks[&self.cursor].terminator.is_some()
    }

    pub fn isTerminated(&self, this: Label<I>) -> bool {
        self.blocks[&this].terminator.is_some()
    }

    pub fn dfs_short_circuit(
        &mut self,
        mut visitor: impl FnMut(&mut Self, Label<I>) -> bool,
    ) -> bool {
        let mut stack = vec![self.entrypoint];
        let mut seen = HashSet::new();

        while let Some(id) = stack.pop() {
            seen.insert(id);

            if visitor(self, id) {
                return true;
            }

            let block = self.blocks.get(&id).unwrap();
            for succ in block.successors.iter() {
                if !seen.contains(succ) && Some(*succ) != block.fallthrough {
                    stack.push(*succ);
                }
            }

            if let Some(ft) = block.fallthrough {
                if !seen.contains(&ft) {
                    stack.push(ft);
                }
            }
        }
        false
    }

    pub fn dfs(&mut self, mut visitor: impl FnMut(&mut Self, Label<I>)) {
        let mut stack = vec![self.entrypoint];
        let mut seen = HashSet::new();

        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }

            visitor(self, id);

            let block = self.blocks.get(&id).unwrap();
            for succ in block.successors.iter() {
                if !seen.contains(succ) {
                    stack.push(*succ);
                }
            }

            // Push the fallthrough block last to ensure its popped off next
            if let Some(ft) = block.fallthrough {
                if !seen.contains(&ft) {
                    stack.push(ft);
                }
            }
        }
    }
}
