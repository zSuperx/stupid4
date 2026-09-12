use crate::common::{BasicBlock, FunctionBuilder, InstructionTrait, Label};
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

#[derive(Default, Debug)]
pub struct ModuleBuilder<I: InstructionTrait, V, T> {
    pub(crate) functions: BTreeMap<Rc<String>, FunctionBuilder<I, V, T>>,
    pub(crate) reg_count: usize,
    pub(crate) block_count: usize,
    pub(crate) cursor: Option<(Rc<String>, Label<I>)>,
}

impl<I: InstructionTrait, V, T> ModuleBuilder<I, V, T> {
    pub fn new() -> Self {
        Self {
            functions: Default::default(),
            reg_count: 0,
            block_count: 0,
            cursor: None,
        }
    }

    pub fn createFunction(
        &mut self,
        name: String,
        return_type: T,
    ) -> &mut FunctionBuilder<I, V, T> {
        let rc = Rc::new(name);
        let f = FunctionBuilder::new(rc.clone(), return_type);
        let None = self.functions.insert(rc.clone(), f) else {
            panic!("Duplicate function");
        };
        // TODO: make this not return anything but instead set a cursor?
        self.functions.get_mut(&rc).unwrap()
    }

    /// Get an iterator over the functions in this module
    pub fn functions(&self) -> impl Iterator<Item = &FunctionBuilder<I, V, T>> {
        self.functions.values()
    }

    /// Get an iterator over mutable functions in this module
    pub fn functions_mut(&mut self) -> impl Iterator<Item = &mut FunctionBuilder<I, V, T>> {
        self.functions.values_mut()
    }

    pub fn emit(&mut self, instr: I) {
        let (function_name, label) = self.cursor.clone().expect("Builder cursor not initialized");
        let function = self.functions.get_mut(&function_name).unwrap();

        let basic_block = function.blocks.get_mut(&label).unwrap();
        if basic_block.terminator.is_some() {
            return;
        }
        if instr.is_terminator() {
            basic_block.terminator = Some(instr);
        } else {
            basic_block.instructions.push(instr);
        }
    }
}
