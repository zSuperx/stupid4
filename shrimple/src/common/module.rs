use crate::common::{FunctionBuilder, InstructionTrait};
use std::collections::{BTreeMap, HashMap};

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct ModuleSymbol(&'static str);

impl std::fmt::Display for ModuleSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Default, Debug)]
pub struct ModuleBuilder<I: InstructionTrait, V, T, M> {
    pub(crate) symbols: HashMap<String, ModuleSymbol>,
    pub(crate) functions: BTreeMap<ModuleSymbol, FunctionBuilder<I, V, T, M>>,
    pub(crate) reg_count: usize,
    pub(crate) block_count: usize,
}

impl<I: InstructionTrait, V, T, M> ModuleBuilder<I, V, T, M> {
    pub fn new() -> Self {
        Self {
            symbols: Default::default(),
            functions: Default::default(),
            reg_count: Default::default(),
            block_count: Default::default(),
        }
    }

    pub fn add_symbol(&mut self, name: String) -> ModuleSymbol {
        let symbol = ModuleSymbol(name.clone().leak());
        self.symbols.insert(name, symbol);
        symbol
    }

    pub fn add_function(&mut self, mut function: FunctionBuilder<I, V, T, M>) {
        let symbol = self
            .symbols
            .get(&function.name)
            .expect("Function symbol has not been added yet");
        function.symbol = *symbol;
        self.functions.insert(*symbol, function);
    }

    /// Get an iterator over the functions in this module
    pub fn functions(&self) -> impl Iterator<Item = &FunctionBuilder<I, V, T, M>> {
        self.functions.values()
    }

    /// Get an iterator over mutable functions in this module
    pub fn functions_mut(&mut self) -> impl Iterator<Item = &mut FunctionBuilder<I, V, T, M>> {
        self.functions.values_mut()
    }
}
