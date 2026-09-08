use clap::{Parser as ArgParser, ValueEnum};
use crate::die;

#[allow(nonstandard_style)]
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Target {
    x86,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    EmitIr,

    EmitAsm,

    CompileOnly,

    AssembleAndLink,
}

pub struct Config {
    pub target: Target,
    pub input: String,
    pub output: String,
    pub action: Action,
    pub verbose: bool,
    pub no_regalloc: bool,
}

#[derive(Debug, ArgParser)]
pub struct Args {
    #[arg(short, long)]
    target: Option<Target>,

    input: String,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(short, long)]
    verbose: bool,

    /// Emit IR, do not compile, assemble, or link
    #[arg(short = 'E')]
    emit_ir: bool,

    /// Compile, do not assemble or link
    #[arg(short = 'S')]
    emit_asm: bool,

    /// Compile and assemble, do not link
    #[arg(short = 'c')]
    compile_only: bool,

    /// Skips the register allocation phase. This is only valid if using -S
    #[arg(long)]
    no_regalloc: bool,
}

pub(super) fn validate_config() -> Config {
    let args = Args::parse();

    let mut action_opt = None;

    if args.emit_ir && action_opt.replace(Action::EmitIr).is_some() {
        die!("Only 1 action can be performed. See -h");
    }

    if args.emit_asm && action_opt.replace(Action::EmitAsm).is_some() {
        die!("Only 1 action can be performed. See -h");
    }

    if args.compile_only && action_opt.replace(Action::CompileOnly).is_some() {
        die!("Only 1 action can be performed. See -h");
    }

    let action = action_opt.unwrap_or(Action::AssembleAndLink);

    let target = args.target.unwrap_or(Target::x86);
    let input = args.input;

    let output = args.output.unwrap_or_else(|| {
        let ext = input.rfind(".");
        let base = input.get(..ext.unwrap_or(input.len())).unwrap();
        match action {
            Action::EmitIr => format!("{base}.ir"),
            Action::EmitAsm => format!("{base}.s"),
            Action::CompileOnly => format!("{base}.o"),
            Action::AssembleAndLink => base.to_string(),
        }
    });
    let verbose = args.verbose;
    let no_regalloc = args.no_regalloc;
    if no_regalloc && action != Action::EmitAsm {
        die!("--no-regalloc can only be used with -S");
    }

    Config {
        target,
        input,
        output,
        action,
        verbose,
        no_regalloc,
    }
}
