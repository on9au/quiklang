//! # Instructions
//!
//! Instructions that are used in the QVM.
//!
//! [`Return to QVM Module`](../index.html)
//!
//! ## Instruction Set
//! TODO: Add instruction set here.

// Instructions

// Rx[n]   : usize = Register n
// Kx[n]   : usize = Constant n
// sBx     : isize = Signed Displacement
// PC      : usize = Program
use std::fmt::Display;

use super::qffi::native_fn::NATIVE_FUNCTION_TABLE;

/// instruction = 32bit(fixed length)
///
/// +---------------------------------------------+
/// |0-5(6bits)|6-13(8bit)|14-22(9bit)|23-31(9bit)|
/// |==========+==========+===========+===========|
/// |  opcode  |    A     |     C     |    B      |
/// |----------+----------+-----------+-----------|
/// |  opcode  |    A     |      Bx(unsigned)     |
/// |----------+----------+-----------+-----------|
/// |  opcode  |    A     |      sBx(signed)      |
/// +---------------------------------------------+

type OpCodeSize = i32;

pub const OPCODE_SIZE: OpCodeSize = 6;
pub const OPCODE_SIZE_A: OpCodeSize = 8;
pub const OPCODE_SIZE_B: OpCodeSize = 9;
pub const OPCODE_SIZE_C: OpCodeSize = 9;
pub const OPCODE_SIZE_BX: OpCodeSize = 18;
pub const OPCODE_SIZE_SBX: OpCodeSize = 18;

pub const OPCODE_MAX_A: OpCodeSize = (1 << OPCODE_SIZE_A) - 1;
pub const OPCODE_MAX_B: OpCodeSize = (1 << OPCODE_SIZE_B) - 1;
pub const OPCODE_MAX_C: OpCodeSize = (1 << OPCODE_SIZE_C) - 1;
pub const OPCODE_MAX_BX: OpCodeSize = (1 << OPCODE_SIZE_BX) - 1;
pub const OPCODE_MAX_SBX: OpCodeSize = OPCODE_MAX_BX >> 1;

// // Opcodes
pub type OpCode = i32;

// // A B      R(A) := R(B)
// pub const OP_MOVE: OpCode = 0;
// // A Bx     R(A) := K(Bx)
// pub const OP_LOADCONST: OpCode = 1;
// // A B C    R(A) := (Bool)B; if (C) pc++
// pub const OP_LOADBOOL: OpCode = 2;
// // A B      R(A) := ... := R(B) := nil
// pub const OP_LOADNULL: OpCode = 3;
// pub const OP_ADD: OpCode = 4;
// pub const OP_SUB: OpCode = 5;
// pub const OP_MUL: OpCode = 6;
// pub const OP_DIV: OpCode = 7;
// pub const OP_MOD: OpCode = 8;
// pub const OP_POW: OpCode = 9;
// pub const OP_NOT: OpCode = 10;
// pub const OP_AND: OpCode = 11;
// pub const OP_OR: OpCode = 12;
// pub const OP_EQ: OpCode = 13;
// pub const OP_NE: OpCode = 14;
// pub const OP_LT: OpCode = 15;
// pub const OP_LE: OpCode = 16;
// pub const OP_GT: OpCode = 17;
// pub const OP_GE: OpCode = 18;
// pub const OP_JUMP: OpCode = 19; // A  PC += A
// pub const OP_JUMP_IF_TRUE: OpCode = 20;
// pub const OP_JUMP_IF_FALSE: OpCode = 21; // A B  if (!R(A)) PC += B
// pub const OP_CALL: OpCode = 22; // A B  Call function at R(A) with B arguments
// pub const OP_NATIVE_CALL: OpCode = 23; // A B  Call VM-Native function at R(A) with B arguments. TODO: A is index to Native Fn registry, then called by QFFI.
// pub const OP_QFFI_CALL: OpCode = 24; // A B  Call FFI (extern) function at R(A) with B arguments. TODO: A is index to FFI registry, then called by QFFI.
// pub const OP_TAILCALL: OpCode = 25;
// pub const OP_RETURN: OpCode = 26; // A  Return with R(A)
// pub const OP_INC: OpCode = 27; // A  R(A)++
// pub const OP_DEC: OpCode = 28; // A  R(A)--
// pub const OP_BITAND: OpCode = 29; // A B C  R(A) := R(B) & R(C)
// pub const OP_BITOR: OpCode = 30; // A B C  R(A) := R(B) | R(C)
// pub const OP_BITXOR: OpCode = 31; // A B C  R(A) := R(B) ^ R(C)
// pub const OP_SHL: OpCode = 32; // A B C  R(A) := R(B) << R(C)
// pub const OP_SHR: OpCode = 33; // A B C  R(A) := R(B) >> R(C)
// pub const OP_CONCAT: OpCode = 34; // A B C  R(A) := RK(B) & RK(C)
// pub const OP_DESTRUCTOR: OpCode = 35; // A B C  R(A) where A is a pointer to heap obj, heap obj is destroyed.
// pub const OP_EXIT: OpCode = 36; // A B C  A is exit code, default 0.
// pub const OP_CLONE: OpCode = 37; // A B  R(A) := R(B).clone()
//                                  // TODO: Add both array, string, hashmap, etc. (Heap-allocated objects) instructions
// pub const OP_ARRAY_ALLOCATE: OpCode = 34; // A B C  R(A) := allocate_array(b - size)
// pub const OP_HASHMAP_ALLOCATE: OpCode = 35; // A B C  R(A) := allocate_hashmap(b - size)
// pub const OP_HASHSET_ALLOCATE: OpCode = 36; // A B C  R(A) := allocate_hashset(b - size)
// pub const OP_STRING_ALLOCATE: OpCode = 37; // A B C  R(A) := allocate_string(b - size)
// pub const OP_ARRAY_OR_HASHSET_PUSH: OpCode = 37; // A B C  array/hashset(A).push(C)
// pub const OP_ARRAY_OR_HASHSET_POP: OpCode = 38; // A B C  R(A) := array/hashset(B).pop()
// pub const OP_GROWABLE_SET: OpCode = 39; // A B C  growable(A)[R(B)].set(C)
// pub const OP_GROWABLE_GET: OpCode = 40; // A B C  R(A) := growable(B)[R(C)]
// pub const OP_HASHMAP_OR_HASHSET_CONTAINS: OpCode = 41; // A B C  R(A) := hashmap/hashset(B).contains(C)
// pub const OP_GROWABLE_REMOVE: OpCode = 42; // A B C  R(A) := growable(B).remove(R(C))
// pub const OP_NOP: OpCode = 38; // NOP

// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// enum OpArgMode {
//     NotUsed,
//     Used,
//     RegisterOrJumpOffset,
//     ConstantOrRegisterConstant,
// }

// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// enum OpType {
//     Abc,
//     ABx,
//     AsBx,
// }

// impl Display for OpType {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match *self {
//             OpType::Abc => write!(f, "Abc"),
//             OpType::ABx => write!(f, "ABx"),
//             OpType::AsBx => write!(f, "AsBx"),
//         }
//     }
// }

// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// pub struct OpProp {
//     name: &'static str,
//     is_test: bool,
//     set_reg_a: bool,
//     mode_arg_b: OpArgMode,
//     mode_arg_c: OpArgMode,
//     typ: OpType,
// }

// pub static OP_NAMES: &[OpProp; OP_NOP as usize + 1] = &[
//     OpProp {
//         name: "MOVE",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::RegisterOrJumpOffset,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "LOADCONST",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::ABx,
//     },
//     OpProp {
//         name: "LOADBOOL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::Used,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "LOADNULL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::RegisterOrJumpOffset,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "ADD",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "SUB",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "MUL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "DIV",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "MOD",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "POW",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "NOT",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::RegisterOrJumpOffset,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "AND",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "OR",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "EQ",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "NE",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "LT",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "LE",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "GT",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "GE",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "JUMP",
//         is_test: false,
//         set_reg_a: false,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::AsBx,
//     },
//     OpProp {
//         name: "JUMP_IF_TRUE",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::AsBx,
//     },
//     OpProp {
//         name: "JUMP_IF_FALSE",
//         is_test: true,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::AsBx,
//     },
//     OpProp {
//         name: "CALL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::Used,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "NATIVE_CALL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "OP_QFFI_CALL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "TAILCALL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::Used,
//         mode_arg_c: OpArgMode::Used,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "RETURN",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::NotUsed,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "INC",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::NotUsed,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "DEC",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::NotUsed,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "BITAND",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "BITOR",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "BITXOR",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "SHL",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "SHR",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "CONCAT",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::ConstantOrRegisterConstant,
//         mode_arg_c: OpArgMode::ConstantOrRegisterConstant,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "DESTRUCTOR",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::NotUsed,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "EXIT",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::NotUsed,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "CLONE",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::RegisterOrJumpOffset,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::Abc,
//     },
//     OpProp {
//         name: "NOP",
//         is_test: false,
//         set_reg_a: true,
//         mode_arg_b: OpArgMode::NotUsed,
//         mode_arg_c: OpArgMode::NotUsed,
//         typ: OpType::AsBx,
//     },
// ];

macro_rules! define_opcodes {
    ($($name:ident => ($op:expr_2021, $is_test:expr_2021, $set_reg_a:expr_2021, $mode_arg_b:expr_2021, $mode_arg_c:expr_2021, $typ:expr_2021)),* $(,)?) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        enum OpArgMode {
            NotUsed,
            Used,
            RegisterOrJumpOffset,
            ConstantOrRegisterConstant,
        }

        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        enum OpType {
            Abc,
            ABx,
            AsBx,
        }

        impl Display for OpType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    OpType::Abc => write!(f, "Abc"),
                    OpType::ABx => write!(f, "ABx"),
                    OpType::AsBx => write!(f, "AsBx"),
                }
            }
        }

        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub struct OpProp {
            name: &'static str,
            is_test: bool,
            set_reg_a: bool,
            mode_arg_b: OpArgMode,
            mode_arg_c: OpArgMode,
            typ: OpType,
        }

        pub static OP_NAMES: &[OpProp] = &[
            $(
                OpProp {
                    name: stringify!($name),
                    is_test: $is_test,
                    set_reg_a: $set_reg_a,
                    mode_arg_b: $mode_arg_b,
                    mode_arg_c: $mode_arg_c,
                    typ: $typ,
                },
            )*
        ];

        $(
            pub const $name: OpCode = $op;
        )*
    };
}

#[rustfmt::skip]
define_opcodes! {
    OP_MOVE =>          (0,     false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_LOADCONST =>     (1,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::NotUsed, OpType::ABx),
    OP_LOADBOOL =>      (2,     false,  true,   OpArgMode::Used, OpArgMode::Used, OpType::Abc),
    OP_LOADNULL =>      (3,     false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_ADD =>       (4,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_SUB =>       (5,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_MUL =>       (6,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_DIV =>       (7,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_MOD =>       (8,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_POW =>       (9,     false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_LOGICAL_NOT =>   (10,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_LOGICAL_AND =>   (11,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_LOGICAL_OR =>    (12,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_EQ =>        (13,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_NE =>        (14,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_LT =>        (15,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_LE =>        (16,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_GT =>        (17,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_GE =>        (18,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_JUMP =>          (19,    false,  false,  OpArgMode::Used, OpArgMode::NotUsed, OpType::ABx),
    OP_JUMP_IF_TRUE =>  (20,    false,  true,   OpArgMode::Used, OpArgMode::NotUsed, OpType::ABx),
    OP_JUMP_IF_FALSE => (21,    false,  true,   OpArgMode::Used, OpArgMode::NotUsed, OpType::ABx),
    OP_CALL =>          (22,    false,  true,   OpArgMode::Used, OpArgMode::Used, OpType::Abc),
    OP_NATIVE_CALL =>   (23,    false,  true,   OpArgMode::Used, OpArgMode::NotUsed, OpType::Abc),
    OP_QFFI_CALL =>     (24,    false,  true,   OpArgMode::Used, OpArgMode::NotUsed, OpType::Abc),
    OP_TAILCALL =>      (25,    false,  true,   OpArgMode::Used, OpArgMode::Used, OpType::Abc),
    OP_RETURN =>        (26,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_INC =>       (27,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_DEC =>       (28,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_BITAND =>    (29,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_BITOR =>     (30,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_BITXOR =>    (31,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_SHL =>       (32,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_SHR =>       (33,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_CONCAT =>        (34,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_DROP_STRING =>   (35,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_EXIT =>          (36,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_CLONE =>         (37,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_BITNOT =>        (38,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_ADD =>     (39,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_SUB =>     (40,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_MUL =>     (41,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_DIV =>     (42,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_POW =>     (43,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_EQ =>      (44,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_NE =>      (45,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_LT =>      (46,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_LE =>      (47,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_GT =>      (48,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_GE =>      (49,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_FLOAT_INC =>     (50,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_DEC =>     (51,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_NEG =>     (52,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_NEG =>       (53,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_TO_FLOAT =>  (54,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_TO_INT =>  (55,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_POSITIVE =>(56,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_INT_POSITIVE =>  (57,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_MOD =>     (58,    false,  true,   OpArgMode::ConstantOrRegisterConstant, OpArgMode::ConstantOrRegisterConstant, OpType::Abc),
    OP_INT_TO_STRING => (59,    false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_FLOAT_TO_STRING=>(60,  false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_DROP_ARRAY =>    (61,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_DROP_RANGE =>    (62,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::Abc),
    OP_MOVE_2X =>       (63,     false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_MOVE_4X =>       (64,     false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_MOVE_8X =>       (65,     false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_MOVE_16X =>      (66,     false,  true,   OpArgMode::RegisterOrJumpOffset, OpArgMode::NotUsed, OpType::Abc),
    OP_NOP =>           (67,    false,  true,   OpArgMode::NotUsed, OpArgMode::NotUsed, OpType::AsBx),
}

pub fn op_to_string(op: i32) -> String {
    OP_NAMES[op as usize].name.to_string()
}

pub type Instruction = u32;

#[inline]
pub fn get_opcode(inst: Instruction) -> OpCode {
    (inst >> 26) as OpCode
}

#[inline]
pub fn set_opcode(inst: &mut Instruction, op: OpCode) {
    *inst = (*inst & 0x3ffffff) | ((op as u32) << 26);
}

#[inline]
pub fn get_arga(inst: Instruction) -> i32 {
    let a = ((inst >> 18) & 0xff) as i32;

    // Convert the 8-bit value to a signed 8-bit integer
    if a & 0x80 != 0 {
        a | !0xff // Extend the sign bit
    } else {
        a
    }
}

#[inline]
pub fn set_arga(inst: &mut Instruction, a: i32) {
    *inst = (*inst & 0xfc03ffff) | ((a as u32 & 0xff) << 18);
}

#[inline]
pub fn get_argb(inst: Instruction) -> i32 {
    let b = (inst & 0x1ff) as i32;
    if b & 0x100 != 0 {
        b | !0x1ff // Extend the sign bit
    } else {
        b
    }
}

#[inline]
pub fn set_argb(inst: &mut Instruction, b: i32) {
    *inst = (*inst & 0xfffffe00) | (b as u32 & 0x1ff);
}

#[inline]
pub fn get_argc(inst: Instruction) -> i32 {
    let c = ((inst >> 9) & 0x1ff) as i32;
    if c & 0x100 != 0 {
        c | !0x1ff // Extend the sign bit
    } else {
        c
    }
}

#[inline]
pub fn set_argc(inst: &mut Instruction, c: i32) {
    *inst = (*inst & 0xfffc01ff) | ((c as u32 & 0x1ff) << 9);
}

#[inline]
pub fn get_argbx(inst: Instruction) -> i32 {
    (inst & 0x3ffff) as i32
}

#[inline]
pub fn set_argbx(inst: &mut Instruction, bx: i32) {
    *inst = (*inst & 0xfffc0000) | (bx as u32 & 0x3ffff);
}

#[inline]
pub fn get_argsbx(inst: Instruction) -> i32 {
    get_argbx(inst) - (OPCODE_MAX_SBX + 1)
}

#[inline]
pub fn set_argsbx(inst: &mut Instruction, sbx: i32) {
    set_argbx(inst, sbx + (OPCODE_MAX_SBX + 1));
}

#[allow(non_snake_case)]
pub fn Abc(op: OpCode, a: i32, b: i32, c: i32) -> Instruction {
    let mut inst: Instruction = 0;
    set_opcode(&mut inst, op);
    set_arga(&mut inst, a);
    set_argb(&mut inst, b);
    set_argc(&mut inst, c);
    inst
}

#[allow(non_snake_case)]
pub fn ABx(op: OpCode, a: i32, bx: i32) -> Instruction {
    let mut inst: Instruction = 0;
    set_opcode(&mut inst, op);
    set_arga(&mut inst, a);
    set_argbx(&mut inst, bx);
    inst
}

#[allow(non_snake_case)]
pub fn ASBx(op: OpCode, a: i32, sbx: i32) -> Instruction {
    let mut inst: Instruction = 0;
    set_opcode(&mut inst, op);
    set_arga(&mut inst, a);
    set_argsbx(&mut inst, sbx);
    inst
}

pub const OP_BIT_RK: OpCodeSize = 1 << (OPCODE_SIZE_B - 1);
pub const OP_MAX_INDEX_RK: OpCodeSize = OP_BIT_RK - 1;

#[inline]
pub fn is_k(v: i32) -> bool {
    v & OP_BIT_RK != 0
}

#[inline]
pub fn rk_ask(v: i32) -> i32 {
    v | OP_BIT_RK
}

#[inline]
pub fn rk_to_k(v: i32) -> i32 {
    (v & 0x1FF) & !OP_BIT_RK
}

pub fn to_string(inst: Instruction) -> String {
    let op = get_opcode(inst);
    if op > OP_NOP {
        return "##### !INVALID OPCODE!".to_string();
    }

    let prop = OP_NAMES[op as usize];
    let arga = get_arga(inst);
    let argb = get_argb(inst);
    let argc = get_argc(inst);
    let argbx = get_argbx(inst);
    let argsbx = get_argsbx(inst);

    let ops = match prop.typ {
        OpType::Abc => format!(
            "{:20}| {:4} | {:4}, {:4}, {:4}",
            prop.name,
            prop.typ.to_string(),
            arga,
            argb,
            argc
        ),
        OpType::ABx => format!(
            "{:20}| {:4} | {:4}, {:10}",
            prop.name,
            prop.typ.to_string(),
            arga,
            argbx
        ),
        OpType::AsBx => format!(
            "{:20}| {:4} | {:4}, {:10}",
            prop.name,
            prop.typ.to_string(),
            arga,
            argsbx
        ),
    };

    let ops = format!("{:36}", ops);

    let format_rk = |v| {
        if is_k(v) {
            format!("K({})", rk_to_k(v))
        } else {
            format!("R({})", v)
        }
    };

    match op {
        OP_MOVE => format!("{} | R({}) := R({})", ops, arga, argb),
        OP_LOADCONST => format!("{} | R({}) := Kst({})", ops, arga, argbx),
        OP_LOADBOOL => format!(
            "{} | R({}) := (Bool){}; if ({}) pc++",
            ops, arga, argb, argc
        ),
        OP_LOADNULL => format!("{} | R({}) := ... := R({}) := nil", ops, arga, argb),
        OP_INT_ADD => format!(
            "{} | R({}) := {} + {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_SUB => format!(
            "{} | R({}) := {} - {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_MUL => format!(
            "{} | R({}) := {} * {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_DIV => format!(
            "{} | R({}) := {} / {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_MOD => format!(
            "{} | R({}) := {} % {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_POW => format!(
            "{} | R({}) := {} ^ {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_LOGICAL_NOT => format!("{} | R({}) := not R({})", ops, arga, argb),
        OP_LOGICAL_AND => format!(
            "{} | R({}) := {} and {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_LOGICAL_OR => format!(
            "{} | R({}) := {} or {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_EQ => format!(
            "{} | R({}) = ({} == {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_NE => format!(
            "{} | R({}) := {} != {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_LT => format!(
            "{} | R({}) = ({} < {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_LE => format!(
            "{} | R({}) = ({} <= {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_GT => format!(
            "{} | R({}) = ({} > {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_GE => format!(
            "{} | R({}) = ({} >= {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_JUMP => format!("{} | pc = {}", ops, argbx),
        OP_JUMP_IF_TRUE => format!("{} | if R({}) is true then pc = {}", ops, arga, argbx),
        OP_JUMP_IF_FALSE => format!("{} | if R({}) is false then pc = {}", ops, arga, argbx),
        OP_CALL => format!("{} | R({})(#R({})), base {}", ops, arga, argb, argc),
        OP_NATIVE_CALL => format!(
            "{} | R({})(#R({})), base {}, NATIVE: {}",
            ops,
            arga,
            argb,
            argc,
            {
                match NATIVE_FUNCTION_TABLE.clone().get(arga as usize) {
                    Some(native_fn) => native_fn.name,
                    None => "UNABLE TO GET FUNCTION!",
                }
            }
        ),
        OP_QFFI_CALL => format!("{} | R({})(#R({})), base {}, QFFI", ops, arga, argb, argc),
        OP_TAILCALL => format!(
            "{} | return R({})(R({}+1) ... R({}+{}-1))",
            ops, arga, arga, arga, argb
        ),
        OP_RETURN => format!("{} | return", ops),
        OP_INT_INC => format!("{} | R({}) += 1", ops, arga),
        OP_INT_DEC => format!("{} | R({}) -= 1", ops, arga),
        OP_INT_BITAND => format!(
            "{} | R({}) := R({}) bitwise and R({})",
            ops, arga, argb, argc
        ),
        OP_INT_BITOR => format!(
            "{} | R({}) := R({}) bitwise or R({})",
            ops, arga, argb, argc
        ),
        OP_INT_BITXOR => format!(
            "{} | R({}) := R({}) bitwise exclusive or R({})",
            ops, arga, argb, argc
        ),
        OP_INT_SHL => format!("{} | R({}) := R({}) << R({})", ops, arga, argb, argc),
        OP_INT_SHR => format!("{} | R({}) := R({}) >> R({})", ops, arga, argb, argc),
        OP_CONCAT => format!("{} | R({}) := R({}) & R({})", ops, arga, argb, argc),
        OP_DROP_STRING => format!("{} | R({}) -> destructor (string)", ops, arga),
        OP_EXIT => format!("{} | std::process::exit({})", ops, arga),
        OP_CLONE => format!("{} | R({}) := R({}).clone()", ops, arga, argb),
        OP_BITNOT => format!("{} | R({}) := ~R({})", ops, arga, argb),
        OP_FLOAT_ADD => format!(
            "{} | R({}) := {} + {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_SUB => format!(
            "{} | R({}) := {} - {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_MUL => format!(
            "{} | R({}) := {} * {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_DIV => format!(
            "{} | R({}) := {} / {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_POW => format!(
            "{} | R({}) := {} ^ {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_EQ => format!(
            "{} | R({}) = ({} == {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_NE => format!(
            "{} | R({}) := {} != {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_LT => format!(
            "{} | R({}) = ({} < {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_LE => format!(
            "{} | R({}) = ({} <= {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_GT => format!(
            "{} | R({}) = ({} > {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_GE => format!(
            "{} | R({}) = ({} >= {})",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_FLOAT_INC => format!("{} | R({}) += 1.0", ops, arga),
        OP_FLOAT_DEC => format!("{} | R({}) -= 1.0", ops, arga),
        OP_FLOAT_NEG => format!("{} | R({}) := -R({})", ops, arga, argb),
        OP_INT_NEG => format!("{} | R({}) := -R({})", ops, arga, argb),
        OP_INT_TO_FLOAT => format!("{} | R({}) := R({}) as float", ops, arga, argb),
        OP_FLOAT_TO_INT => format!("{} | R({}) := R({}) as int", ops, arga, argb),
        OP_FLOAT_POSITIVE => format!("{} | R({}) := +R({})", ops, arga, argb),
        OP_INT_POSITIVE => format!("{} | R({}) := +R({})", ops, arga, argb),
        OP_FLOAT_MOD => format!(
            "{} | R({}) := {} % {}",
            ops,
            arga,
            format_rk(argb),
            format_rk(argc)
        ),
        OP_INT_TO_STRING => format!("{} | R({}) := R({}).to_string()", ops, arga, argb),
        OP_FLOAT_TO_STRING => format!("{} | R({}) := R({}).to_string()", ops, arga, argb),
        OP_DROP_ARRAY => format!("{} | R({}) -> destructor (array)", ops, arga),
        OP_DROP_RANGE => format!("{} | R({}) -> destructor (range)", ops, arga),
        OP_MOVE_2X => format!(
            "{} | R({}) ..= R({}) := R({}) ..= R({})",
            ops,
            arga,
            arga + 1,
            argb,
            argb + 1
        ),
        OP_MOVE_4X => format!(
            "{} | R({}) ..= R({}) := R({}) ..= R({})",
            ops,
            arga,
            arga + 3,
            argb,
            argb + 3
        ),
        OP_MOVE_8X => format!(
            "{} | R({}) ..= R({}) := R({}) ..= R({})",
            ops,
            arga,
            arga + 7,
            argb,
            argb + 7
        ),
        OP_MOVE_16X => format!(
            "{} | R({}) ..= R({}) := R({}) ..= R({})",
            ops,
            arga,
            arga + 15,
            argb,
            argb + 15
        ),
        OP_NOP => ops.to_string(),
        _ => unreachable!(),
    }
}
