// VM (Register-based)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{oneshot, Mutex};

use crate::{bytecode::TaggedConstantValue, errors::VMRuntimeError};

use super::{
    bytecode::ByteCode,
    instructions::{
        get_arga, get_argb, get_argbx, get_argc, get_opcode, is_k, rk_to_k, Instruction, OP_NOP,
    },
    qffi::QFFI,
    register_val::RegisterVal,
};

type VmHandler = fn(&mut VMThread, Instruction) -> Result<(), VMRuntimeError>;

#[repr(C, align(2))]
#[derive(Debug, Clone, Copy)]
struct CallFrame {
    pub return_pc: u32, // PC to return to
    pub base: u32,      // Base register
}

const fn create_dispatch_table() -> [VmHandler; OP_NOP as usize + 1] {
    [
        VMThread::op_move,
        VMThread::op_loadconst,
        VMThread::op_loadbool,
        VMThread::op_loadnull,
        VMThread::op_int_add,
        VMThread::op_int_sub,
        VMThread::op_int_mul,
        VMThread::op_int_div,
        VMThread::op_int_mod,
        VMThread::op_int_pow,
        VMThread::op_logical_not,
        VMThread::op_logical_and,
        VMThread::op_logical_or,
        VMThread::op_int_eq,
        VMThread::op_int_ne,
        VMThread::op_int_lt,
        VMThread::op_int_le,
        VMThread::op_int_gt,
        VMThread::op_int_ge,
        VMThread::op_jump,
        VMThread::op_jump_if_true,
        VMThread::op_jump_if_false,
        VMThread::op_call,
        VMThread::op_native_call,
        VMThread::op_qffi_call,
        VMThread::op_tailcall,
        VMThread::op_return,
        VMThread::op_int_inc,
        VMThread::op_int_dec,
        VMThread::op_int_bitand,
        VMThread::op_int_bitor,
        VMThread::op_int_bitxor,
        VMThread::op_int_shl,
        VMThread::op_int_shr,
        VMThread::op_concat,
        VMThread::op_drop_string,
        VMThread::op_exit,
        VMThread::op_clone,
        VMThread::op_bitnot,
        VMThread::op_float_add,
        VMThread::op_float_sub,
        VMThread::op_float_mul,
        VMThread::op_float_div,
        VMThread::op_float_pow,
        VMThread::op_float_eq,
        VMThread::op_float_ne,
        VMThread::op_float_lt,
        VMThread::op_float_le,
        VMThread::op_float_gt,
        VMThread::op_float_ge,
        VMThread::op_float_inc,
        VMThread::op_float_dec,
        VMThread::op_float_neg,
        VMThread::op_int_neg,
        VMThread::op_int_to_float,
        VMThread::op_float_to_int,
        VMThread::op_float_positive,
        VMThread::op_int_positive,
        VMThread::op_float_mod,
        VMThread::op_int_to_string,
        VMThread::op_float_to_string,
        VMThread::op_drop_array,
        VMThread::op_drop_range,
        VMThread::op_move_2x,
        VMThread::op_move_4x,
        VMThread::op_move_8x,
        VMThread::op_move_16x,
        VMThread::op_nop,
    ]
}

static DISPATCH_TABLE: [VmHandler; OP_NOP as usize + 1] = create_dispatch_table();

// The global state, which is the VM itself. Holds immutable, shared data, which local states (VMThread) can access.
#[derive(Debug)]
#[repr(C)]
pub struct VM {
    constant_pool: Box<[RegisterVal]>,   // Constant pool
    constant_ptrs: HashSet<RegisterVal>, // Pointers to heap allocated objects
    function_indexes: Vec<usize>,        // Function indexes
    instructions: Vec<Instruction>,      // Instructions
    qffi: QFFI,                          // QFFI instance
    threads: Mutex<HashMap<usize, tokio::task::JoinHandle<()>>>, // Active threads
    thread_id_counter: Mutex<usize>,     // For assigning unique thread IDs
                                         // TODO: Add a symbol table for global variables
                                         // TODO: MPSC channel for inter-thread communication
}

pub struct VMThread {
    thread_id: usize,              // Thread ID
    program_counter: usize,        // Local PC
    vm: Arc<VM>,                   // Global state (VM)
    registers: Box<[RegisterVal]>, // Local registers
    call_stack: Box<[CallFrame]>,  // Local call stack
    call_stack_pointer: usize,     // Local stack pointer
}

const ARRAY_REPEAT_VALUE: RegisterVal = RegisterVal { null: () };

impl VM {
    // Create a new VM instance
    pub fn new(
        instructions: Vec<Instruction>,
        constant_pool: Vec<RegisterVal>,
        constant_ptrs: HashSet<RegisterVal>,
        function_indexes: Vec<usize>,
        // main_fn_max_reg: usize,
    ) -> Self {
        VM {
            constant_pool: constant_pool.into_boxed_slice(),
            constant_ptrs,
            instructions,
            function_indexes,
            qffi: QFFI::new(),
            threads: Mutex::new(HashMap::new()),
            thread_id_counter: Mutex::new(0),
        }
    }

    // Create a new VM instance from a bytecode
    pub fn from_bytecode(bytecode: ByteCode) -> Self {
        let mut constant_ptrs: HashSet<RegisterVal> = HashSet::new();
        let constant_pool = bytecode
            .constant_pool()
            .clone()
            .iter()
            .map(|f| match f {
                TaggedConstantValue::Null => RegisterVal { null: () },
                TaggedConstantValue::Int(int) => RegisterVal { int: *int },
                TaggedConstantValue::Float(float) => RegisterVal { float: *float },
                TaggedConstantValue::Bool(bool) => RegisterVal { bool: *bool },
                TaggedConstantValue::Str(string) => {
                    let ptr = RegisterVal {
                        ptr: RegisterVal::set_ptr_from_value::<String>(string.to_string()),
                    };

                    // Add the pointer to the constant_ptrs set
                    constant_ptrs.insert(ptr);

                    ptr
                }
            })
            .collect();

        VM::new(
            bytecode.instructions().clone(),
            constant_pool,
            constant_ptrs,
            bytecode
                .qlang_functions
                .iter()
                .map(|f| *f as usize)
                .collect::<Vec<usize>>()
                .clone(),
            // *bytecode.register_count() as usize,
        )
    }

    // Async API to spawn a new thread (VMThread) and execute it
    #[inline(always)]
    pub async fn spawn_thread(&mut self, mut vm_thread: VMThread) -> Result<usize, VMRuntimeError> {
        let (tx, rx) = oneshot::channel();

        // TODO: Optimize this, as it is not efficient to lock the mutex every time a thread is spawned
        let thread_id = {
            let mut counter = self.thread_id_counter.lock().await;
            *counter += 1;
            *counter
        };

        vm_thread.thread_id = thread_id;

        let handle = tokio::spawn(async move {
            let result = vm_thread.execute().await; // Execute thread's instructions
            let _ = tx.send(result); // Send result back to parent
        });

        // Store thread in the thread manager
        self.threads.lock().await.insert(thread_id, handle);

        Ok(thread_id)
    }

    // Async API to check if a thread has finished and get the result
    #[inline(always)]
    pub async fn join_thread(&mut self, thread_id: usize) -> Result<(), VMRuntimeError> {
        match self.threads.lock().await.remove(&thread_id) { Some(handle) => {
            handle
                .await
                .map_err(|e| VMRuntimeError::ThreadJoinError(thread_id, e))?;
            Ok(())
        } _ => {
            Err(VMRuntimeError::InvalidThreadId(thread_id))
        }}
    }

    // API to see if a pointer is a constant pointer
    #[inline(always)]
    pub fn is_constant_ptr(&self, ptr: &RegisterVal) -> bool {
        self.constant_ptrs.contains(ptr)
    }

    // API to fetch a constant from the constant pool
    #[inline(always)]
    pub fn get_constant(&self, index: usize) -> Option<&RegisterVal> {
        self.constant_pool.get(index)
    }

    // API to fetch constant pool len
    #[inline(always)]
    pub fn get_constant_pool_len(&self) -> usize {
        self.constant_pool.len()
    }

    // API to fetch an instruction from the instruction pool
    #[inline(always)]
    pub fn get_instruction(&self, pc: usize) -> Option<&Instruction> {
        self.instructions.get(pc)
    }

    // API to fetch instruction pool len
    #[inline(always)]
    pub fn get_instruction_len(&self) -> usize {
        self.instructions.len()
    }

    // API to access Native QFFI functions
    #[inline(always)]
    pub fn call_qffi_native(
        &self,
        index: usize,
        args: &[RegisterVal],
    ) -> Result<RegisterVal, VMRuntimeError> {
        self.qffi.call_native_function(index, args)
    }

    // API to access Extern QFFI functions
    #[inline(always)]
    pub fn call_qffi_extern(
        &self,
        index: usize,
        args: &[RegisterVal],
    ) -> Result<RegisterVal, VMRuntimeError> {
        self.qffi.call_extern_function(index, args)
    }

    // API to fetch function index
    #[inline(always)]
    pub fn get_function_index(&self, fn_id: usize) -> Option<&usize> {
        self.function_indexes.get(fn_id)
    }

    // API to fetch function index len
    #[inline(always)]
    pub fn get_function_index_len(&self) -> usize {
        self.function_indexes.len()
    }

    // Async API to process exit code
    // We need to terminate all threads.
    // Propagating the exit code to the caller of VM.execute() is done by the thread that called this function.
    #[inline(always)]
    pub async fn process_exit(&self) {
        // Terminate all threads
        for (_, handle) in self.threads.lock().await.drain() {
            handle.abort();
        }
    }

    // Execute the VM
    // This function simply initializes a new VMThread and executes it.
    pub async fn execute(self) -> Result<RegisterVal, VMRuntimeError> {
        let mut vm_thread = VMThread::new(Arc::new(self));
        vm_thread.execute().await
    }
}

impl VMThread {
    pub fn new(vm: Arc<VM>) -> Self {
        VMThread {
            thread_id: 0,
            program_counter: 0,
            vm,
            registers: Box::new([ARRAY_REPEAT_VALUE; 20000]), // DefaultL 20000
            call_stack: Box::new(
                [CallFrame {
                    return_pc: 0,
                    base: 0,
                }; 30000], // Default: 30000
            ),
            call_stack_pointer: 0,
        }
    }

    fn push_call_frame(&mut self, frame: CallFrame) -> Result<(), VMRuntimeError> {
        // self.call_stack.push(frame);
        if self.call_stack_pointer >= self.call_stack.len() {
            return Err(VMRuntimeError::StackOverflow);
        }
        self.call_stack[self.call_stack_pointer] = frame;
        self.call_stack_pointer += 1;
        Ok(())
    }

    fn pop_call_frame(&mut self) -> Result<CallFrame, VMRuntimeError> {
        if self.call_stack_pointer == 0 {
            return Err(VMRuntimeError::StackUnderflow);
        }
        // let frame = self.call_stack.pop().unwrap();
        let frame = self.call_stack[self.call_stack_pointer - 1];
        self.call_stack_pointer -= 1;
        Ok(frame)
    }

    #[inline]
    fn current_offset(&mut self) -> usize {
        if self.call_stack_pointer == 0 {
            return 0;
        }
        self.call_stack[self.call_stack_pointer - 1].base as usize
    }

    // fn current_frame(&self) -> &CallFrame {
    //     &self.call_stack[self.stack_pointer - 1]
    // }

    #[inline(always)]
    fn fetch_instruction(&self) -> Instruction {
        // self.instructions[self.program_counter]
        *self.vm.get_instruction(self.program_counter).unwrap()
    }

    // This on_err function unwinds the callstack, displaying the user the last 20
    // calls, and popping all of the frames on the stack.
    pub fn on_err_unwind_callstack(&mut self) {
        println!("Unwinding Callstack:");
        println!("Stack pointer is at: {}", self.call_stack_pointer);
        if self.call_stack_pointer > 41 {
            println!("Truncating callstack to first and last 20 calls...");
        }
        let mut count: usize = 0;
        let mut reverse_count: usize = self.call_stack_pointer;
        while let Ok(call_frame) = self.pop_call_frame() {
            count += 1;
            if count <= 20 {
                println!(
                    "Return PC: {:6} | Base: {}",
                    call_frame.return_pc, call_frame.base
                )
            }

            if reverse_count == 21 {
                println!("...");
            }

            if reverse_count <= 20 {
                println!(
                    "Return PC: {:6} | Base: {}",
                    call_frame.return_pc, call_frame.base
                )
            }
            reverse_count -= 1;
        }
        println!("END");
    }

    // This on_err function "bulldoses" (destructs) heap allocated objects to prevent memory leaking.
    // This is partically useful in cases when Rc is not used, and there are other processes (QLBPM) running.
    pub fn on_err_bulldoser(&mut self) {
        println!("Running memory bulldoser; destroying heap objects:");
        // TODO: This would run a destructor function (either OP_DESTRUCTOR or dedicated function)
        // to destroy all heap allocated objects.
        println!("END");
    }

    pub fn on_error_cleanup(&mut self) {
        self.on_err_unwind_callstack();
        println!();
        self.on_err_bulldoser();
    }

    #[inline(always)]
    fn set_register(&mut self, register: usize, value: RegisterVal) -> Result<(), VMRuntimeError> {
        // When tinyvec is used, this will allocate memory for the register.
        let offset = self.current_offset();
        if register + offset >= self.registers.len() {
            // self.registers
            //     .resize(register + offset + 1, RegisterVal::Null);
            return Err(VMRuntimeError::InvalidRegisterAccess(register + offset));
        }
        self.registers[register + offset] = value;
        Ok(())
    }

    #[inline(always)]
    fn set_multiple_registers(
        &mut self,
        base_register: usize,
        values: &[RegisterVal],
        offset: usize,
    ) -> Result<(), VMRuntimeError> {
        if base_register + offset + values.len() >= self.registers.len() {
            return Err(VMRuntimeError::InvalidRegisterAccess(
                base_register + offset,
            ));
        }
        for (i, value) in values.iter().enumerate() {
            self.registers[base_register + offset + i] = *value;
        }
        Ok(())
    }

    #[inline(always)]
    fn get_register_mutref(&mut self, register: usize) -> Result<&mut RegisterVal, VMRuntimeError> {
        let offset = self.current_offset();
        if register + offset >= self.registers.len() {
            return Err(VMRuntimeError::InvalidRegisterAccess(register + offset));
        }
        Ok(&mut self.registers[register + offset])
    }

    #[inline(always)]
    fn get_register_ref(
        &self,
        register: usize,
        offset: usize,
    ) -> Result<&RegisterVal, VMRuntimeError> {
        if register + offset >= self.registers.len() {
            return Err(VMRuntimeError::InvalidRegisterAccess(register + offset));
        }
        Ok(&self.registers[register + offset])
    }

    #[inline(always)]
    fn get_multiple_registers_ref(
        &self,
        base_register: usize,
        count: usize,
        offset: usize,
    ) -> Result<&[RegisterVal], VMRuntimeError> {
        if base_register + offset + count >= self.registers.len() {
            return Err(VMRuntimeError::InvalidRegisterAccess(
                base_register + offset,
            ));
        }
        Ok(&self.registers[base_register + offset..base_register + offset + count])
    }

    #[inline(always)]
    fn get_register_clone(&mut self, register: usize) -> Result<RegisterVal, VMRuntimeError> {
        let offset = self.current_offset();
        if register + offset >= self.registers.len() {
            return Err(VMRuntimeError::InvalidRegisterAccess(register + offset));
        }
        Ok(self.registers[register + offset])
    }

    #[inline(always)]
    fn get_constant_clone(&mut self, index: usize) -> Result<RegisterVal, VMRuntimeError> {
        self.vm
            .get_constant(index)
            .cloned()
            .ok_or(VMRuntimeError::InvalidConstantAccess(index))
    }

    #[inline(always)]
    fn get_constant_ref(&self, index: usize) -> Result<&RegisterVal, VMRuntimeError> {
        self.vm
            .get_constant(index)
            .ok_or(VMRuntimeError::InvalidConstantAccess(index))
    }

    #[inline(always)]
    fn on_thread_failure(&mut self, error: VMRuntimeError) -> VMRuntimeError {
        println!("Thread ID: {} failed with error: {}", self.thread_id, error);
        self.on_error_cleanup();
        error
    }

    pub async fn execute(&mut self) -> Result<RegisterVal, VMRuntimeError> {
        // The result is typically register[0], which is the return value of the function. This is important for async functions.
        let inst_len = self.vm.get_instruction_len();
        while self.program_counter < inst_len {
            let inst = self.fetch_instruction();
            self.program_counter += 1;
            let opcode = get_opcode(inst);
            if opcode as usize >= DISPATCH_TABLE.len() {
                return Err(VMRuntimeError::InvalidOpcode(opcode, self.program_counter));
            }
            match DISPATCH_TABLE[opcode as usize](self, inst) {
                // The instruction was successful, so we continue to the next instruction
                Ok(_) => {}
                // Process exited, so we need to interrupt the execution and return the exit code
                Err(VMRuntimeError::Exit(code)) => {
                    self.vm.process_exit().await;
                    return Err(VMRuntimeError::Exit(code));
                }
                // The thread failed, so we should unwind the callstack and clean up the memory
                Err(e) => return Err(self.on_thread_failure(e)),
            };
        }
        Ok(*self.registers.first().unwrap_or(&RegisterVal { null: () }))
    }

    #[inline(always)]
    fn op_move(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        // let value = std::mem::take(self.get_register_mutref(argb as usize)?);
        let value = self.get_register_ref(argb as usize, offset)?;
        self.set_register(arga as usize, *value)?;

        Ok(())
    }

    #[inline(always)]
    fn op_loadconst(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argbx = get_argbx(inst);

        let value = self.get_constant_clone(argbx as usize)?;
        self.set_register(arga as usize, value)?;

        Ok(())
    }

    #[inline(always)]
    fn op_loadbool(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);

        let value = if argb != 0 {
            RegisterVal { bool: true }
        } else {
            RegisterVal { bool: false }
        };
        self.set_register(arga as usize, value)?;
        if argc != 0 {
            self.program_counter += 1;
        }

        Ok(())
    }

    #[inline(always)]
    fn op_loadnull(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);

        for i in arga as usize..=argb as usize {
            self.set_register(i, RegisterVal { null: () })?;
        }

        Ok(())
    }

    #[inline(always)]
    fn fetch_values(
        &self,
        argb: i32,
        argc: i32,
        offset: usize,
    ) -> Result<(&RegisterVal, &RegisterVal), VMRuntimeError> {
        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };
        Ok((b_val, c_val))
    }

    #[inline(always)]
    fn op_int_add(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_op(inst, |left, right| Ok(left.wrapping_add(right)))
    }

    #[inline(always)]
    fn op_int_sub(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_op(inst, |left, right| Ok(left.wrapping_sub(right)))
    }

    #[inline(always)]
    fn op_int_mul(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_op(inst, |left, right| Ok(left.wrapping_mul(right)))
    }

    #[inline(always)]
    fn op_int_div(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_op(inst, |left, right| {
            if right == 0 {
                return Err(VMRuntimeError::DivideByZero);
            }
            Ok(left.wrapping_div(right))
        })
    }

    #[inline(always)]
    fn op_int_mod(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_op(inst, |left, right| Ok(left.wrapping_rem(right)))
    }

    #[inline(always)]
    fn op_int_pow(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_op(inst, |left, right| Ok(left.wrapping_pow(right as u32)))
    }

    #[inline(always)]
    fn perform_int_op<FInt>(
        &mut self,
        inst: Instruction,
        int_op: FInt,
    ) -> Result<(), VMRuntimeError>
    where
        FInt: FnOnce(i64, i64) -> Result<i64, VMRuntimeError>,
    {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let (b_val, c_val) = self.fetch_values(argb, argc, offset)?;

        let result = int_op(unsafe { b_val.int }, unsafe { c_val.int })?;

        self.set_register(arga as usize, RegisterVal { int: result })?;

        Ok(())
    }

    #[inline(always)]
    fn perform_float_op<FFloat>(
        &mut self,
        inst: Instruction,
        float_op: FFloat,
    ) -> Result<(), VMRuntimeError>
    where
        FFloat: FnOnce(f64, f64) -> Result<f64, VMRuntimeError>,
    {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let (b_val, c_val) = self.fetch_values(argb, argc, offset)?;

        let result = float_op(unsafe { b_val.float }, unsafe { c_val.float })?;

        self.set_register(arga as usize, RegisterVal { float: result })?;

        Ok(())
    }

    #[inline(always)]
    fn op_logical_not(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                bool: !unsafe { b_val.bool },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_logical_and(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                bool: unsafe { b_val.bool } && unsafe { c_val.bool },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_logical_or(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                bool: unsafe { b_val.bool } || unsafe { c_val.bool },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn perform_int_comparison<F>(
        &mut self,
        inst: Instruction,
        int_comp: F,
    ) -> Result<(), VMRuntimeError>
    where
        F: FnOnce(i64, i64) -> bool,
    {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let (b_val, c_val) = self.fetch_values(argb, argc, offset)?;

        self.set_register(
            arga as usize,
            RegisterVal {
                bool: int_comp(unsafe { b_val.int }, unsafe { c_val.int }),
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn perform_float_comparison<F>(
        &mut self,
        inst: Instruction,
        float_comp: F,
    ) -> Result<(), VMRuntimeError>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let (b_val, c_val) = self.fetch_values(argb, argc, offset)?;

        self.set_register(
            arga as usize,
            RegisterVal {
                bool: float_comp(unsafe { b_val.float }, unsafe { c_val.float }),
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_eq(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_comparison(inst, |b, c| b == c)
    }

    #[inline(always)]
    fn op_int_ne(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_comparison(inst, |b, c| b != c)
    }

    #[inline(always)]
    fn op_int_lt(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_comparison(inst, |b, c| b < c)
    }

    #[inline(always)]
    fn op_int_le(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_comparison(inst, |b, c| b <= c)
    }

    #[inline(always)]
    fn op_int_gt(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_comparison(inst, |b, c| b > c)
    }

    #[inline(always)]
    fn op_int_ge(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_int_comparison(inst, |b, c| b >= c)
    }

    #[inline(always)]
    fn op_jump(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let argbx = get_argbx(inst);

        self.program_counter = argbx as usize;

        Ok(())
    }

    #[inline(always)]
    fn op_jump_if_true(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argbx = get_argbx(inst);
        let offset = self.current_offset();

        if unsafe { self.get_register_ref(arga as usize, offset)?.bool } {
            self.program_counter = argbx as usize;
        }

        Ok(())
    }

    #[inline(always)]
    fn op_jump_if_false(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argbx = get_argbx(inst);
        let offset = self.current_offset();

        if !unsafe { self.get_register_ref(arga as usize, offset)?.bool } {
            self.program_counter = argbx as usize;
        }

        Ok(())
    }

    #[inline(always)]
    fn op_call(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        // let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        // A is the pointer to a function on function_indexes
        // B is the number of args to be pushed into the function's scope
        // C is the base

        // Save callframe
        let call_frame = CallFrame {
            return_pc: self.program_counter as u32,
            base: (argc as u32) + offset as u32,
        };

        // push callframe
        self.push_call_frame(call_frame)?;

        // Set the program counter to the function's starting instruction
        if (arga as usize) < self.vm.get_function_index_len() {
            self.program_counter = *self.vm.get_function_index(arga as usize).unwrap();
        } else {
            panic!("Invalid function index: {}", arga);
        }

        Ok(())
    }

    #[inline(always)]
    fn op_native_call(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        // Native calls rely on the QFFI module.
        // Args = (offset + argc + 1) to (offset + argb + argc)
        let mut args = Vec::with_capacity(argc as usize + 1);
        for i in ((argc as usize) + offset + 1)..=(((argc + argb) as usize) + offset) {
            args.push(self.registers[i]);
        }

        self.set_register(
            argc as usize,
            self.vm.call_qffi_native(arga as usize, &args)?,
        )
    }

    #[inline(always)]
    fn op_qffi_call(&mut self, _inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!()
    }

    #[inline(always)]
    fn op_tailcall(&mut self, _inst: Instruction) -> Result<(), VMRuntimeError> {
        // TODO!
        todo!()
    }

    #[inline(always)]
    fn op_return(&mut self, _inst: Instruction) -> Result<(), VMRuntimeError> {
        self.program_counter = self.pop_call_frame()?.return_pc as usize;

        Ok(())
    }

    #[inline(always)]
    fn op_int_inc(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);

        let value = self.get_constant_ref(arga as usize)?;

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { value.int }.wrapping_add(1),
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_dec(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);

        let value = self.get_constant_ref(arga as usize)?;

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { value.int }.wrapping_sub(1),
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_bitand(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { b_val.int } & unsafe { c_val.int },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_bitor(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { b_val.int } | unsafe { c_val.int },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_bitxor(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { b_val.int } ^ unsafe { c_val.int },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_shl(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { b_val.int } << unsafe { c_val.int },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_int_shr(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        self.set_register(
            arga as usize,
            RegisterVal {
                int: unsafe { b_val.int } >> unsafe { c_val.int },
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_concat(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let argc = get_argc(inst);
        let offset = self.current_offset();

        let b_val = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };
        let c_val = if is_k(argc) {
            self.get_constant_ref(rk_to_k(argc) as usize)?
        } else {
            self.get_register_ref(argc as usize, offset)?
        };

        let b_val_str = b_val.get_value_from_ptr::<String>()?;
        let c_val_str = c_val.get_value_from_ptr::<String>()?;

        // Concatenate the strings
        let concatenated = format!("{}{}", b_val_str, c_val_str);

        // Obtain a raw pointer to the boxed string
        let ptr = RegisterVal::set_ptr_from_value::<String>(concatenated);

        self.set_register(arga as usize, RegisterVal { ptr })?;

        Ok(())
    }

    #[inline(always)]
    fn op_drop_string(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let offset = self.current_offset();

        // Get the value in the register
        let value = self.get_register_ref(arga as usize, offset)?;

        // Check if the pointer is a constant
        if self.vm.is_constant_ptr(value) {
            return Ok(());
        }

        // Pointer is not a constant, so we need to drop it
        value.drop_ptr::<String>();

        Ok(())
    }

    #[inline(always)]
    fn op_exit(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);

        Err(VMRuntimeError::Exit(arga))
    }

    #[inline(always)]
    fn op_clone(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let value = self.get_register_ref(argb as usize, offset)?;
        self.set_register(arga as usize, *value)?;

        Ok(())
    }

    #[inline(always)]
    fn op_bitnot(&mut self, _inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!("OP_BITNOT")
    }

    #[inline(always)]
    fn op_float_add(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_op(inst, |left, right| Ok(left + right))
    }

    #[inline(always)]
    fn op_float_sub(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_op(inst, |left, right| Ok(left - right))
    }

    #[inline(always)]
    fn op_float_mul(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_op(inst, |left, right| Ok(left * right))
    }

    #[inline(always)]
    fn op_float_div(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_op(inst, |left, right| {
            if right == 0.0 {
                return Err(VMRuntimeError::DivideByZero);
            }
            Ok(left / right)
        })
    }

    #[inline(always)]
    fn op_float_pow(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_op(inst, |left, right| Ok(left.powf(right)))
    }

    #[inline(always)]
    fn op_float_eq(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_comparison(inst, |b, c| b == c)
    }

    #[inline(always)]
    fn op_float_ne(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_comparison(inst, |b, c| b != c)
    }

    #[inline(always)]
    fn op_float_lt(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_comparison(inst, |b, c| b < c)
    }

    #[inline(always)]
    fn op_float_le(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_comparison(inst, |b, c| b <= c)
    }

    #[inline(always)]
    fn op_float_gt(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_comparison(inst, |b, c| b > c)
    }

    #[inline(always)]
    fn op_float_ge(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_comparison(inst, |b, c| b >= c)
    }

    #[inline(always)]
    fn op_float_inc(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);

        let value = self.get_constant_ref(arga as usize)?;

        self.set_register(
            arga as usize,
            RegisterVal {
                float: unsafe { value.float } + 1.0,
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_float_dec(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);

        let value = self.get_constant_ref(arga as usize)?;

        self.set_register(
            arga as usize,
            RegisterVal {
                float: unsafe { value.float } - 1.0,
            },
        )?;

        Ok(())
    }

    #[inline(always)]
    fn op_float_neg(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!("OP_FLOAT_NEG")
    }

    #[inline(always)]
    fn op_int_neg(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!("OP_INT_NEG")
    }

    #[inline(always)]
    fn op_int_to_float(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let value = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };

        let float = unsafe { value.int } as f64;

        self.set_register(arga as usize, RegisterVal { float })?;

        Ok(())
    }

    #[inline(always)]
    fn op_float_to_int(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let value = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };

        let int = unsafe { value.float } as i64;

        self.set_register(arga as usize, RegisterVal { int })?;

        Ok(())
    }

    #[inline(always)]
    fn op_float_positive(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!("OP_FLOAT_POSITIVE")
    }

    #[inline(always)]
    fn op_int_positive(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!("OP_INT_POSITIVE")
    }

    #[inline(always)]
    fn op_float_mod(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        self.perform_float_op(inst, |left, right| Ok(left % right))
    }

    #[inline(always)]
    fn op_int_to_string(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let value = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };

        let string = format!("{}", unsafe { value.int });

        let ptr = RegisterVal::set_ptr_from_value::<String>(string);

        self.set_register(arga as usize, RegisterVal { ptr })?;

        Ok(())
    }

    #[inline(always)]
    fn op_float_to_string(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let value = if is_k(argb) {
            self.get_constant_ref(rk_to_k(argb) as usize)?
        } else {
            self.get_register_ref(argb as usize, offset)?
        };

        let string = format!("{}", unsafe { value.float });

        let ptr = RegisterVal::set_ptr_from_value::<String>(string);

        self.set_register(arga as usize, RegisterVal { ptr })?;

        Ok(())
    }

    #[inline(always)]
    fn op_drop_array(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);

        let value = self.get_register_ref(arga as usize, 0)?;

        value.drop_ptr::<Vec<RegisterVal>>();

        Ok(())
    }

    #[inline(always)]
    fn op_drop_range(&mut self, _inst: Instruction) -> Result<(), VMRuntimeError> {
        todo!("OP_DROP_RANGE")
    }

    #[inline(always)]
    fn op_move_2x(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let values = self
            .get_multiple_registers_ref(argb as usize, 2, offset)?
            .to_vec();

        self.set_multiple_registers(arga as usize, &values, offset)?;

        Ok(())
    }

    #[inline(always)]
    fn op_move_4x(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let values = self
            .get_multiple_registers_ref(argb as usize, 4, offset)?
            .to_vec();

        self.set_multiple_registers(arga as usize, &values, offset)?;

        Ok(())
    }

    #[inline(always)]
    fn op_move_8x(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let values = self
            .get_multiple_registers_ref(argb as usize, 8, offset)?
            .to_vec();

        self.set_multiple_registers(arga as usize, &values, offset)?;

        Ok(())
    }

    #[inline(always)]
    fn op_move_16x(&mut self, inst: Instruction) -> Result<(), VMRuntimeError> {
        let arga = get_arga(inst);
        let argb = get_argb(inst);
        let offset = self.current_offset();

        let values = self
            .get_multiple_registers_ref(argb as usize, 16, offset)?
            .to_vec();

        self.set_multiple_registers(arga as usize, &values, offset)?;

        Ok(())
    }

    #[inline(always)]
    fn op_nop(&mut self, _inst: Instruction) -> Result<(), VMRuntimeError> {
        // here only to waste cycles lol
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::{VMThread, VM};

    #[tokio::test]
    async fn sizes() {
        println!("Size of VM: {} bytes", std::mem::size_of::<VM>());
        println!(
            "Size of VMThread: {} bytes",
            std::mem::size_of::<VMThread>()
        );
    }
}
