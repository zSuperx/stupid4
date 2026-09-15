#!/usr/bin/env perl

use strict;
use warnings;
use FindBin qw($Bin);

require "$Bin/utils/printers.pl";

# Meta-constants: Rust types used in ISA definition
my $VAL = "x86Value";
my $INSTR = "x86Instr";
my $LABEL = "x86Label";

# templates
my $accumOp = {
  args => [ "rs1:$VAL", "rs2:$VAL" ],
  uses => [ "rs1", "rs2" ],
  defs => [ "rs1" ],
	fmt  => "{name} {rs1}, {rs2}",
  term => 0,
};

my $jcc = {
  args => [ "to:$LABEL" ],
  fmt => "{name} {to}",
  term => 1,
};

my $move = {
  args => [ "dst:$VAL", "rs1:$VAL" ],
  uses => [ "rs1" ],
  defs => [ "dst" ],
  fmt => "{name} {dst}, {rs1}",
};

# isa spec
my $isa = {
  isaName => "x86",
  instrName => $INSTR,
  value => $VAL,

  instructions => {
    comment => {
      args => [ "s:String" ],
      fmt  => "; {s}",
    },

    add => $accumOp,

    sub => $accumOp,

    imul => $accumOp,

    idiv => $accumOp,

    # TODO: add this later when you figure out wtf it does
    # mul => { 
    #   args = [ "rs1:$VAL" ],
    #   uses = [ "RAX", "rs1" ],
    #   defs = [ "RAX", "rs1" ],
    #   fmt = "imul {rs1}",
    # },

    cmp => {
      args => [ "rs1:$VAL", "rs2:$VAL" ],
      uses => [ "rs1", "rs2" ],
    },

    jl  => $jcc,
    jle => $jcc,
    jg  => $jcc,
    jge => $jcc,
    je  => $jcc,
    jne => $jcc,
    jz  => $jcc,
    jnz => $jcc,
    jo  => $jcc,
    jno => $jcc,
    jge => $jcc,
    jmp => $jcc,
    ret => { term => 1 },

    lea    => $move,
    mov    => $move,
    movzx  => $move,
    movsx  => $move,
    movsxd => $move,
    cmove  => $move,
    cmovne => $move,
    cmovl  => $move,
    cmovle => $move,
    cmovg  => $move,
    cmovge => $move,

    pop  => {
      args => [ "dst:$VAL" ],
      defs => [ "dst" ],
    },

    push => {
      args => [ "rs1:$VAL" ],
      uses => [ "rs1" ],
    },

    call => {
      args => [ "rs1:$VAL" ],
      uses => [ "rs1" ],
      defs => [ "&Reg(RAX)" ],
    },
  },

  extraCode => [
    "use crate::target::x86::isa::*;",
    "use x86Value::*;",
    "use crate::target::x86::builder::*;",
  ],
  specFile => __FILE__,
};

printAll($isa);
