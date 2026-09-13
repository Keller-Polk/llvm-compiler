use inkwell::OptimizationLevel;
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::types::BasicType;
use inkwell::types::{AnyTypeEnum, BasicTypeEnum};
use inkwell::values::BasicValueEnum;
use inkwell::values::FunctionValue;
use inkwell::values::GlobalValue;
use inkwell::values::PointerValue;
use inkwell::values::{BasicValue, InstructionValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::Path;
use std::process::Command;

use crate::lexer::*;
use crate::parser::*;

pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub symbol_table: RefCell<Vec<HashMap<String, (PointerValue<'ctx>, Type)>>>,
    pub loop_stack: RefCell<Vec<LoopContext<'ctx>>>,
}

#[derive(Clone, Copy)]
pub struct LoopContext<'ctx> {
    pub continue_block: BasicBlock<'ctx>,
    pub break_block: BasicBlock<'ctx>,
}

pub enum Components<'ctx> {
    Function(FunctionValue<'ctx>),
    Global(GlobalValue<'ctx>),
}

impl<'ctx> Codegen<'ctx> {
    // Call this when entering a new function or block
    pub fn push_scope(&self) {
        self.symbol_table.borrow_mut().push(HashMap::new());
    }

    // Call this when exiting a function or block
    pub fn pop_scope(&self) {
        self.symbol_table
            .borrow_mut()
            .pop()
            .expect("Compiler Error: Scope underflow.");
    }

    pub fn push_loop(&self, continue_block: BasicBlock<'ctx>, break_block: BasicBlock<'ctx>) {
        self.loop_stack.borrow_mut().push(LoopContext {
            continue_block,
            break_block,
        });
    }

    pub fn pop_loop(&self) {
        self.loop_stack
            .borrow_mut()
            .pop()
            .expect("Compiler Error: Loop stack underflow.");
    }

    pub fn current_loop(&self) -> LoopContext<'ctx> {
        *self
            .loop_stack
            .borrow()
            .last()
            .expect("Compiler Error: break/continue used outside of a loop.")
    }

    pub fn current_block_has_terminator(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(|block| block.get_terminator())
            .is_some()
    }
}

impl Program {
    pub fn build(&self, name: &str) {
        // init
        Target::initialize_native(&InitializationConfig::default())
            .expect("Compiler Error: Failed to initialize native LLVM target.");

        let context = Context::create();
        let module = context.create_module("my_compiler");
        let builder = context.create_builder();

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .expect("Compiler Error: Failed to get native LLVM target.");
        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                OptimizationLevel::None,
                RelocMode::Default,
                CodeModel::Default,
            )
            .expect("Compiler Error: Failed to create LLVM target machine.");

        module.set_triple(&target_triple);
        module.set_data_layout(&target_machine.get_target_data().get_data_layout());

        let code_gen = Codegen {
            context: &context,
            module: module,
            builder: builder,
            symbol_table: std::cell::RefCell::new(Vec::new()),
            loop_stack: std::cell::RefCell::new(Vec::new()),
        };

        declareBuiltinFunctions(&code_gen);

        // 1. Create a hidden dummy function frame
        let void_type = context.void_type();
        let dummy_fn_type = void_type.fn_type(&[], false);
        let dummy_func = code_gen
            .module
            .add_function("__global_init_sandbox", dummy_fn_type, None);

        // 2. Append an empty basic block and force position the cursor inside it
        let sandbox_block = context.append_basic_block(dummy_func, "sandbox");
        code_gen.builder.position_at_end(sandbox_block);

        ///
        for x in self.items.clone() {
            let item = match x {
                Item::Global(global_declaration) => Ok(Components::Global(
                    global_declaration.buildGlobal(&code_gen),
                )),
                Item::Function(func_def) => Ok(Components::Function(
                    func_def
                        .buildFunction(&code_gen)
                        .expect("Compiler Error: Failed to build function."),
                )),
                _ => {
                    Err("Compiler Error: top-level item must be a function or global declaration.")
                }
            };
        }

        unsafe {
            dummy_func.delete();
        }

        buildMainWrapper(&code_gen);

        code_gen
            .module
            .verify()
            .expect("Compiler Error: Generated LLVM module failed verification.");

        compileModule(&code_gen, &target_machine, name);
    }
}

fn buildMainWrapper<'ctx>(code_gen: &Codegen<'ctx>) {
    if code_gen.module.get_function("main").is_some() {
        return;
    }

    let run_function = match code_gen.module.get_function("run") {
        Some(function) => function,
        None => return,
    };

    let i32_type = code_gen.context.i32_type();
    let main_type = i32_type.fn_type(&[], false);
    let main_function = code_gen.module.add_function("main", main_type, None);
    let entry_block = code_gen.context.append_basic_block(main_function, "entry");

    code_gen.builder.position_at_end(entry_block);
    code_gen
        .builder
        .build_call(run_function, &[], "run_call")
        .unwrap();

    let zero = i32_type.const_int(0, false);
    code_gen.builder.build_return(Some(&zero)).unwrap();
}

fn compileModule<'ctx>(code_gen: &Codegen<'ctx>, target_machine: &TargetMachine, name: &str) {
    let object_path = Path::new("output.o");
    let executable_path = Path::new(name);

    target_machine
        .write_to_file(&code_gen.module, FileType::Object, object_path)
        .expect("Compiler Error: Failed to write object file.");

    let status = Command::new("cc")
        .arg("-no-pie")
        .arg(object_path)
        .arg("-o")
        .arg(executable_path)
        .status()
        .expect("Compiler Error: Failed to run system linker 'cc'.");

    if !status.success() {
        panic!("Compiler Error: Linking failed with status {:?}.", status);
    }
}

// #[derive(Debug, Clone,PartialEq)]
// pub struct FunctionDecl {
//     pub name: String,
//     pub params: Vec<Param>,
//     pub return_type: Type,
//     pub body: Vec<Stmt>,
// }

impl FunctionDecl {
    fn buildFunction<'ctx>(&self, code_gen: &Codegen<'ctx>) -> Result<FunctionValue<'ctx>, String> {
        let context = &code_gen.context;
        let module = &code_gen.module;
        let builder = &code_gen.builder;

        code_gen.push_scope();
        let mut arg_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
        for param in &self.params {
            let llvm_type = param.ty.to_llvm_type(&code_gen);
            let basic_type: inkwell::types::BasicTypeEnum = llvm_type
                .try_into()
                .map_err(|_| "Void is not allowed as a parameter type.")?;

            arg_types.push(BasicMetadataTypeEnum::from(basic_type));
        }
        let return_type = self.return_type.to_llvm_type(&code_gen);

        let fn_type = match return_type {
            inkwell::types::AnyTypeEnum::VoidType(void_ty) => void_ty.fn_type(&arg_types, false),
            inkwell::types::AnyTypeEnum::IntType(int_ty) => int_ty.fn_type(&arg_types, false),
            inkwell::types::AnyTypeEnum::FloatType(float_ty) => float_ty.fn_type(&arg_types, false),
            inkwell::types::AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.fn_type(&arg_types, false),
            _ => return Err("Unsupported function return type.".to_string()),
        };

        let function = module.add_function(&self.name, fn_type, None);

        let entry_block = context.append_basic_block(function, "entry");
        builder.position_at_end(entry_block);

        for (i, arg) in function.get_param_iter().enumerate() {
            let param_name = &self.params[i].name;
            let param_type = self.params[i].ty.clone();
            let param_type_llvm = arg.get_type();

            let alloca = builder.build_alloca(param_type_llvm, param_name).unwrap();

            builder.build_store(alloca, arg).unwrap();

            let mut table_ref = code_gen.symbol_table.borrow_mut();

            let current_scope = table_ref
                .last_mut()
                .expect("Compiler Error: No active function compilation scope found.");

            current_scope.insert(param_name.clone(), (alloca, param_type));
        }

        for x in self.body.clone() {
            if code_gen.current_block_has_terminator() {
                break;
            }

            x.buildLine(&code_gen);
        }

        let current_block = code_gen.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            let return_type = self.return_type.to_llvm_type(&code_gen);

            match return_type {
                inkwell::types::AnyTypeEnum::VoidType(_) => {
                    code_gen.builder.build_return(None).unwrap();
                }
                inkwell::types::AnyTypeEnum::IntType(int_ty) => {
                    let default_zero = int_ty.const_int(0, false);
                    code_gen.builder.build_return(Some(&default_zero)).unwrap();
                }
                inkwell::types::AnyTypeEnum::FloatType(float_ty) => {
                    let default_zero = float_ty.const_float(0.0);
                    code_gen.builder.build_return(Some(&default_zero)).unwrap();
                }
                inkwell::types::AnyTypeEnum::PointerType(ptr_ty) => {
                    let default_null = ptr_ty.const_null();
                    code_gen.builder.build_return(Some(&default_null)).unwrap();
                }
                _ => {
                    return Err(
                        "Missing explicit return statement for this return type.".to_string()
                    );
                }
            };
        }
        code_gen.pop_scope();
        Ok(function)
    }
}
impl Stmt {
    fn buildLine<'ctx>(self, code_gen: &Codegen<'ctx>) {
        match self {
            Stmt::Assignment(x) => {
                x.buildAssignment(&code_gen);
            }
            Stmt::Return(x) => {
                buildReturn(&x, &code_gen);
            }
            Stmt::Expr(x) => {
                x.buildExprs(&code_gen);
            }
            Stmt::If(x) => {
                x.buildIf(&code_gen);
            }
            Stmt::While(x) => {
                x.buildWhile(&code_gen);
            }
            Stmt::Break => {
                buildBreak(&code_gen);
            }
            Stmt::Continue => {
                buildContinue(&code_gen);
            }
        };
    }
}

impl IfStmt {
    fn buildIf<'ctx>(&self, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
        let context = &code_gen.context;
        let module = &code_gen.module;
        let builder = &code_gen.builder;

        let (condition, _cond_ty) = self.condition.llvm_expr(code_gen).unwrap();

        let int_cond = condition.into_int_value();
        let current_function = builder.get_insert_block().unwrap().get_parent().unwrap();

        let then_block = context.append_basic_block(current_function, "then_branch");
        let else_block = context.append_basic_block(current_function, "else_branch");
        let merge_block = context.append_basic_block(current_function, "if_merge");

        let branch_inst = builder
            .build_conditional_branch(int_cond, then_block, else_block)
            .unwrap();

        builder.position_at_end(then_block);

        for x in self.then_branch.clone() {
            if code_gen.current_block_has_terminator() {
                break;
            }

            x.buildLine(&code_gen);
        }

        if builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            builder.build_unconditional_branch(merge_block).unwrap();
        }

        builder.position_at_end(else_block);

        code_gen.push_scope();

        if let Some(else_branch) = &self.else_branch {
            for x in else_branch.clone() {
                if code_gen.current_block_has_terminator() {
                    break;
                }

                x.buildLine(&code_gen);
            }
        }

        code_gen.pop_scope();

        if builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            builder.build_unconditional_branch(merge_block).unwrap();
        }

        builder.position_at_end(merge_block);

        branch_inst
    }
}
/*#[derive(Debug, Clone,PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Vec<Stmt>,
    pub else_branch: Option<Vec<Stmt>>,
}*/

impl WhileStmt {
    fn buildWhile<'ctx>(&self, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
        let context = &code_gen.context;
        let module = &code_gen.module;
        let builder = &code_gen.builder;

        let current_function = builder.get_insert_block().unwrap().get_parent().unwrap();

        let cond_block = context.append_basic_block(current_function, "while_cond");
        let body_block = context.append_basic_block(current_function, "while_body");
        let end_block = context.append_basic_block(current_function, "while_end");

        let entry_branch = builder.build_unconditional_branch(cond_block).unwrap();

        builder.position_at_end(cond_block);

        let (cond_val, _cond_ty) = self.condition.llvm_expr(code_gen).unwrap();
        let int_cond = cond_val.into_int_value();

        builder
            .build_conditional_branch(int_cond, body_block, end_block)
            .unwrap();

        builder.position_at_end(body_block);
        code_gen.push_scope();
        code_gen.push_loop(cond_block, end_block);

        for x in self.body.clone() {
            if code_gen.current_block_has_terminator() {
                break;
            }

            x.buildLine(&code_gen);
        }

        code_gen.pop_loop();
        code_gen.pop_scope();
        if builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            builder.build_unconditional_branch(cond_block).unwrap();
        }

        builder.position_at_end(end_block);

        entry_branch
    }
}

fn index_to_i64<'ctx>(
    index: BasicValueEnum<'ctx>,
    code_gen: &Codegen<'ctx>,
) -> inkwell::values::IntValue<'ctx> {
    let index_int = index.into_int_value();
    let i64_type = code_gen.context.i64_type();

    if index_int.get_type().get_bit_width() == 64 {
        index_int
    } else {
        code_gen
            .builder
            .build_int_z_extend(index_int, i64_type, "index_i64")
            .unwrap()
    }
}

fn buildIndexPointer<'ctx>(
    array_expr: &Expr,
    index_expr: &Expr,
    code_gen: &Codegen<'ctx>,
) -> Result<(PointerValue<'ctx>, Type), String> {
    let (array_value, array_ty) = array_expr.llvm_expr(code_gen)?;
    let (index_value, _index_ty) = index_expr.llvm_expr(code_gen)?;
    let index = index_to_i64(index_value, code_gen);

    let inner_ty = match array_ty {
        Type::Pointer(inner_ty) => *inner_ty,
        other => return Err(format!("indexing expects a pointer, got {:?}", other)),
    };

    let inner_llvm_type: BasicTypeEnum = inner_ty
        .to_llvm_type(code_gen)
        .try_into()
        .expect("Compiler Error: indexed pointer cannot point to void.");

    let element_ptr = unsafe {
        code_gen
            .builder
            .build_gep(
                inner_llvm_type,
                array_value.into_pointer_value(),
                &[index],
                "index_ptr",
            )
            .unwrap()
    };

    Ok((element_ptr, inner_ty))
}

impl AssignmentStmt {
    fn buildAssignment<'ctx>(&self, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
        let context = &code_gen.context;
        let module = &code_gen.module;
        let builder = &code_gen.builder;

        if let Some(index) = &self.index {
            let array_expr = Expr::Operand(Token::Identifier(self.name.clone()));
            let (target_ptr, inner_ty) = buildIndexPointer(&array_expr, index, code_gen)
                .expect("Compiler Error: Failed to build indexed assignment pointer.");
            let (evaluated_val, value_ty) = self
                .value
                .clone()
                .expect("Compiler Error: indexed assignment must have a value.")
                .llvm_expr(code_gen)
                .expect("Compiler Error: Failed to compile indexed assignment value.");

            if value_ty != inner_ty {
                panic!(
                    "Compiler Error: cannot store value of type {:?} into indexed pointer of type {:?}.",
                    value_ty, inner_ty
                );
            }

            return builder.build_store(target_ptr, evaluated_val).unwrap();
        }

        if self.dereference {
            let (ptr, pointer_ty) = {
                let table_ref = code_gen.symbol_table.borrow();
                table_ref
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(&self.name))
                    .cloned()
                    .expect(&format!("{:?} has not been defined.", &self.name))
            };

            let inner_ty = match pointer_ty.clone() {
                Type::Pointer(inner_ty) => *inner_ty,
                _ => panic!(
                    "Compiler Error: cannot dereference non-pointer variable {:?}.",
                    self.name
                ),
            };

            let pointer_llvm_type: BasicTypeEnum = pointer_ty
                .to_llvm_type(&code_gen)
                .try_into()
                .expect("Compiler Error: pointer variable should have an LLVM basic type.");

            let target_ptr = builder
                .build_load(pointer_llvm_type, ptr, &self.name)
                .unwrap()
                .into_pointer_value();

            let (evaluated_val, value_ty) = self
                .value
                .clone()
                .expect("Compiler Error: pointer stores must have a value.")
                .llvm_expr(code_gen)
                .expect("Compiler Error: Failed to compile pointer store value.");

            if value_ty != inner_ty {
                panic!(
                    "Compiler Error: cannot store value of type {:?} into pointer to {:?}.",
                    value_ty, inner_ty
                );
            }

            return builder.build_store(target_ptr, evaluated_val).unwrap();
        }

        let llvm_any_type = match self.ty.clone() {
            Some(x) => x.to_llvm_type(&code_gen),
            None => {
                let (_, ty) = {
                    let table_ref = code_gen.symbol_table.borrow();
                    table_ref
                        .iter()
                        .rev()
                        .find_map(|scope| scope.get(&self.name))
                        .cloned()
                        .expect(&format!("{:?} has not been defined.", &self.name))
                };
                ty.to_llvm_type(&code_gen)
            }
        };

        let llvm_basic_type: inkwell::types::BasicTypeEnum = llvm_any_type
            .try_into()
            .expect("Compiler Error: Local variables cannot be initialized with Void types.");

        let local_var = builder.build_alloca(llvm_basic_type, &self.name).unwrap();

        let (evaluated_val, value_ty) = self
            .value
            .clone()
            .expect("Compiler Error: Local variables must be initialized with a value.")
            .llvm_expr(code_gen)
            .expect("Compiler Error: Failed to compile local variable initializer.");

        let stored_ty = self.ty.clone().unwrap_or(value_ty);
        let store_instruction = builder.build_store(local_var, evaluated_val).unwrap();

        let mut table_ref = code_gen.symbol_table.borrow_mut();
        let current_scope = table_ref
            .last_mut()
            .expect("Compiler Error: No active compilation scope found.");

        if !current_scope.contains_key(&self.name) {
            current_scope.insert(self.name.clone(), (local_var, stored_ty));
        }
        store_instruction
    }
}
impl Expr {
    fn buildExprs<'ctx>(&self, code_gen: &Codegen<'ctx>) {
        if let Expr::Operand(Token::Call(name, args)) = self {
            if name == "print" {
                buildPrint(args, code_gen);
                return;
            }
            if name == "free" {
                buildFree(args, code_gen);
                return;
            }
            if name == "array_set" {
                buildArraySet(args, code_gen);
                return;
            }
            if name == "push" {
                buildPush(args, code_gen);
                return;
            }
        }

        self.llvm_expr(code_gen)
            .expect("Compiler Error: Failed to codegen standalone expression line.");
    }
}

fn declareBuiltinFunctions<'ctx>(code_gen: &Codegen<'ctx>) {
    getPrintf(code_gen);
    getMalloc(code_gen);
    getFree(code_gen);
}

fn getPrintf<'ctx>(code_gen: &Codegen<'ctx>) -> FunctionValue<'ctx> {
    if let Some(function) = code_gen.module.get_function("printf") {
        return function;
    }

    let i32_type = code_gen.context.i32_type();
    let ptr_type = code_gen.context.ptr_type(inkwell::AddressSpace::default());
    let printf_type = i32_type.fn_type(&[ptr_type.into()], true);

    code_gen.module.add_function("printf", printf_type, None)
}

fn getMalloc<'ctx>(code_gen: &Codegen<'ctx>) -> FunctionValue<'ctx> {
    if let Some(function) = code_gen.module.get_function("malloc") {
        return function;
    }

    let i64_type = code_gen.context.i64_type();
    let ptr_type = code_gen.context.ptr_type(inkwell::AddressSpace::default());
    let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);

    code_gen.module.add_function("malloc", malloc_type, None)
}

fn getFree<'ctx>(code_gen: &Codegen<'ctx>) -> FunctionValue<'ctx> {
    if let Some(function) = code_gen.module.get_function("free") {
        return function;
    }

    let ptr_type = code_gen.context.ptr_type(inkwell::AddressSpace::default());
    let free_type = code_gen
        .context
        .void_type()
        .fn_type(&[ptr_type.into()], false);

    code_gen.module.add_function("free", free_type, None)
}

fn buildPrint<'ctx>(args: &Vec<Expr>, code_gen: &Codegen<'ctx>) {
    if args.len() != 1 {
        panic!("Compiler Error: print expects exactly one argument.");
    }

    let (mut value, ty) = args[0]
        .llvm_expr(code_gen)
        .expect("Compiler Error: Failed to compile print argument.");

    let format_string = match ty {
        Type::I8
        | Type::I16
        | Type::I32
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::Bool
        | Type::Char => "%d\n",
        Type::I64 | Type::U64 => "%lu\n",
        Type::F32 => {
            let f64_type = code_gen.context.f64_type();
            let extended = code_gen
                .builder
                .build_float_ext(value.into_float_value(), f64_type, "print_f64_tmp")
                .unwrap();
            value = extended.into();
            "%f\n"
        }
        Type::F64 => "%f\n",
        Type::Pointer(_) => "%p\n",
        Type::Array(_, _) => panic!("Compiler Error: print cannot print arrays directly yet."),
        Type::Void => panic!("Compiler Error: print cannot print void."),
    };

    let printf = getPrintf(code_gen);
    let format_ptr = code_gen
        .builder
        .build_global_string_ptr(format_string, "print_format")
        .unwrap()
        .as_pointer_value();

    code_gen
        .builder
        .build_call(printf, &[format_ptr.into(), value.into()], "print_call")
        .unwrap();
}

fn buildFree<'ctx>(args: &Vec<Expr>, code_gen: &Codegen<'ctx>) {
    if args.len() != 1 {
        panic!("Compiler Error: free expects exactly one argument.");
    }

    let (ptr_value, ptr_ty) = args[0]
        .llvm_expr(code_gen)
        .expect("Compiler Error: Failed to compile free argument.");

    match ptr_ty {
        Type::Pointer(_) => {}
        _ => panic!("Compiler Error: free expects a pointer, got {:?}.", ptr_ty),
    }

    let free = getFree(code_gen);
    code_gen
        .builder
        .build_call(free, &[ptr_value.into()], "free_call")
        .unwrap();
}

fn buildArraySet<'ctx>(args: &Vec<Expr>, code_gen: &Codegen<'ctx>) {
    if args.len() != 3 {
        panic!("Compiler Error: array_set expects array, index, and value.");
    }

    let (target_ptr, inner_ty) = buildIndexPointer(&args[0], &args[1], code_gen)
        .expect("Compiler Error: Failed to build array_set pointer.");
    let (value, value_ty) = args[2]
        .llvm_expr(code_gen)
        .expect("Compiler Error: Failed to compile array_set value.");

    if value_ty != inner_ty {
        panic!(
            "Compiler Error: cannot array_set value of type {:?} into array of {:?}.",
            value_ty, inner_ty
        );
    }

    code_gen.builder.build_store(target_ptr, value).unwrap();
}

fn buildArrayGet<'ctx>(
    args: &Vec<Expr>,
    code_gen: &Codegen<'ctx>,
) -> Result<(BasicValueEnum<'ctx>, Type), String> {
    if args.len() != 2 {
        return Err("Compiler Error: array_get expects array and index.".to_string());
    }

    let (element_ptr, inner_ty) = buildIndexPointer(&args[0], &args[1], code_gen)?;
    let inner_llvm_type: BasicTypeEnum = inner_ty
        .to_llvm_type(code_gen)
        .try_into()
        .expect("Compiler Error: array_get pointer cannot point to void.");
    let loaded_val = code_gen
        .builder
        .build_load(inner_llvm_type, element_ptr, "array_get_load")
        .unwrap();

    Ok((loaded_val, inner_ty))
}

fn buildPop<'ctx>(
    args: &Vec<Expr>,
    code_gen: &Codegen<'ctx>,
) -> Result<(BasicValueEnum<'ctx>, Type), String> {
    if args.len() != 2 {
        return Err("Compiler Error: pop expects array and length pointer.".to_string());
    }

    let (len_ptr_value, len_ptr_ty) = args[1].llvm_expr(code_gen)?;
    let len_inner_ty = match len_ptr_ty {
        Type::Pointer(inner_ty) => *inner_ty,
        other => {
            return Err(format!(
                "Compiler Error: pop length argument must be a pointer, got {:?}.",
                other
            ));
        }
    };
    let len_llvm_type: BasicTypeEnum = len_inner_ty
        .to_llvm_type(code_gen)
        .try_into()
        .expect("Compiler Error: pop length pointer cannot point to void.");
    let len_ptr = len_ptr_value.into_pointer_value();
    let len_value = code_gen
        .builder
        .build_load(len_llvm_type, len_ptr, "pop_len")
        .unwrap()
        .into_int_value();
    let one = len_value.get_type().const_int(1, false);
    let new_len = code_gen
        .builder
        .build_int_sub(len_value, one, "pop_new_len")
        .unwrap();
    code_gen.builder.build_store(len_ptr, new_len).unwrap();

    let (array_value, array_ty) = args[0].llvm_expr(code_gen)?;
    let inner_ty = match array_ty {
        Type::Pointer(inner_ty) => *inner_ty,
        other => {
            return Err(format!(
                "Compiler Error: pop array argument must be a pointer, got {:?}.",
                other
            ));
        }
    };
    let inner_llvm_type: BasicTypeEnum = inner_ty
        .to_llvm_type(code_gen)
        .try_into()
        .expect("Compiler Error: pop array cannot point to void.");
    let index = index_to_i64(new_len.into(), code_gen);
    let element_ptr = unsafe {
        code_gen
            .builder
            .build_gep(
                inner_llvm_type,
                array_value.into_pointer_value(),
                &[index],
                "pop_ptr",
            )
            .unwrap()
    };
    let loaded_val = code_gen
        .builder
        .build_load(inner_llvm_type, element_ptr, "pop_value")
        .unwrap();

    Ok((loaded_val, inner_ty))
}

fn buildPush<'ctx>(args: &Vec<Expr>, code_gen: &Codegen<'ctx>) {
    if args.len() != 3 {
        panic!("Compiler Error: push expects array, length pointer, and value.");
    }

    let (len_ptr_value, len_ptr_ty) = args[1]
        .llvm_expr(code_gen)
        .expect("Compiler Error: Failed to compile push length pointer.");
    let len_inner_ty = match len_ptr_ty {
        Type::Pointer(inner_ty) => *inner_ty,
        other => panic!(
            "Compiler Error: push length argument must be a pointer, got {:?}.",
            other
        ),
    };
    let len_llvm_type: BasicTypeEnum = len_inner_ty
        .to_llvm_type(code_gen)
        .try_into()
        .expect("Compiler Error: push length pointer cannot point to void.");
    let len_ptr = len_ptr_value.into_pointer_value();
    let len_value = code_gen
        .builder
        .build_load(len_llvm_type, len_ptr, "push_len")
        .unwrap()
        .into_int_value();

    let (target_ptr, inner_ty) =
        buildIndexPointer(&args[0], &Expr::Operand(Token::Int(0)), code_gen)
            .expect("Compiler Error: Failed to validate push array pointer.");
    let _ = target_ptr;
    let (array_value, array_ty) = args[0]
        .llvm_expr(code_gen)
        .expect("Compiler Error: Failed to compile push array.");
    let array_inner_ty = match array_ty {
        Type::Pointer(inner_ty) => *inner_ty,
        other => panic!(
            "Compiler Error: push array argument must be a pointer, got {:?}.",
            other
        ),
    };
    if array_inner_ty != inner_ty {
        panic!("Compiler Error: push internal type mismatch.");
    }
    let inner_llvm_type: BasicTypeEnum = array_inner_ty
        .to_llvm_type(code_gen)
        .try_into()
        .expect("Compiler Error: push array cannot point to void.");
    let index = index_to_i64(len_value.into(), code_gen);
    let element_ptr = unsafe {
        code_gen
            .builder
            .build_gep(
                inner_llvm_type,
                array_value.into_pointer_value(),
                &[index],
                "push_ptr",
            )
            .unwrap()
    };

    let (value, value_ty) = args[2]
        .llvm_expr(code_gen)
        .expect("Compiler Error: Failed to compile push value.");
    if value_ty != array_inner_ty {
        panic!(
            "Compiler Error: cannot push value of type {:?} into array of {:?}.",
            value_ty, array_inner_ty
        );
    }
    code_gen.builder.build_store(element_ptr, value).unwrap();

    let one = len_value.get_type().const_int(1, false);
    let new_len = code_gen
        .builder
        .build_int_add(len_value, one, "push_new_len")
        .unwrap();
    code_gen.builder.build_store(len_ptr, new_len).unwrap();
}

fn buildReturn<'ctx>(expr: &Option<Expr>, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
    let builder = &code_gen.builder;

    if expr.is_none() {
        builder.build_return(None).unwrap()
    } else {
        let (local_var, _ty) = expr
            .clone()
            .expect("Compiler Error: Local variables must be initialized with a default value.")
            .llvm_expr(code_gen)
            .expect("Failed to codegen return expression");

        builder.build_return(Some(&local_var)).unwrap()
    }
}

fn buildBreak<'ctx>(code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
    let loop_context = code_gen.current_loop();
    code_gen
        .builder
        .build_unconditional_branch(loop_context.break_block)
        .unwrap()
}

fn buildContinue<'ctx>(code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
    let loop_context = code_gen.current_loop();
    code_gen
        .builder
        .build_unconditional_branch(loop_context.continue_block)
        .unwrap()
}

impl GlobalDecl {
    fn buildGlobal<'ctx>(&self, code_gen: &Codegen<'ctx>) -> GlobalValue<'ctx> {
        let context = &code_gen.context;
        let module = &code_gen.module;
        let builder = &code_gen.builder;

        let llvm_any_type = self.ty.to_llvm_type(code_gen);

        let llvm_basic_type: inkwell::types::BasicTypeEnum = llvm_any_type
            .try_into()
            .expect("Compiler Error: Global variables cannot be initialized with Void types.");

        let global_var = module.add_global(llvm_basic_type, None, &self.name);
        global_var.set_linkage(inkwell::module::Linkage::External);

        let (evaluated_val, ty) = self
            .value
            .clone()
            .expect("Compiler Error: Global variables must be initialized with a value.")
            .llvm_expr(code_gen)
            .expect("Compiler Error: Failed to compile global variable initializer.");

        if self.pointer && !evaluated_val.is_pointer_value() && !evaluated_val.is_int_value() {
            std::panic::panic_any(
                "Compiler Error: Cannot initialize a global pointer with a floating-point value.",
            );
        }

        let default_initializer: inkwell::values::BasicValueEnum<'ctx> = if self.pointer {
            if evaluated_val.is_pointer_value() {
                evaluated_val
            } else {
                let int_val = evaluated_val.into_int_value();
                let ptr_type = code_gen.context.ptr_type(inkwell::AddressSpace::from(0));

                int_val.const_to_pointer(ptr_type).into()
            }
        } else {
            evaluated_val
        };

        global_var.set_initializer(&default_initializer);

        global_var
    }
}

// #[derive(Debug, Clone,PartialEq)]
// pub struct GlobalDecl {
//     pub name: String,
//     pub ty: Type,
//     pub value: Option<Expr>,
//     pub pointer: bool,
// }

impl Type {
    pub fn is_signed(&self) -> bool {
        match self {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 => true,
            _ => false, // All U variants, Bool, Char, and Arrays evaluate to false
        }
    }

    fn to_llvm_type<'ctx>(&self, code_gen: &Codegen<'ctx>) -> AnyTypeEnum<'ctx> {
        let context = &code_gen.context;
        let module = &code_gen.module;
        let builder = &code_gen.builder;
        match self {
            Type::I8 => context.i8_type().into(),
            Type::I16 => context.i16_type().into(),
            Type::I32 => context.i32_type().into(),
            Type::I64 => context.i64_type().into(),

            Type::U8 => context.i8_type().into(),
            Type::U16 => context.i16_type().into(),
            Type::U32 => context.i32_type().into(),
            Type::U64 => context.i64_type().into(),

            Type::F32 => context.f32_type().into(),
            Type::F64 => context.f64_type().into(),

            Type::Bool => context.custom_width_int_type(1).into(),

            Type::Char => context.i8_type().into(),

            Type::Void => context.void_type().into(),

            Type::Array(inner_type, size) => {
                let any_inner = inner_type.to_llvm_type(code_gen);

                let basic_inner: BasicTypeEnum = any_inner
                    .try_into()
                    .expect("Compiler Error: Cannot generate an LLVM array of Void components.");

                basic_inner.array_type(*size).into()
            }
            Type::Pointer(inner_type) => {
                let any_inner = inner_type.to_llvm_type(code_gen);

                let basic_inner: BasicTypeEnum = any_inner
                    .try_into()
                    .expect("Compiler Error: Cannot create a pointer to Void.");

                basic_inner
                    .ptr_type(inkwell::AddressSpace::default())
                    .into()
            }
        }
    }
    fn from_llvm_type(opt_llvm_ty: Option<BasicTypeEnum>) -> Self {
        let llvm_ty = match opt_llvm_ty {
            None => return Type::Void,
            Some(ty) => ty,
        };

        match llvm_ty {
            BasicTypeEnum::IntType(int_ty) => match int_ty.get_bit_width() {
                1 => Type::Bool,
                8 => Type::I8,
                16 => Type::I16,
                32 => Type::I32,
                64 => Type::I64,
                _ => panic!("Unsupported LLVM integer bit-width"),
            },
            BasicTypeEnum::FloatType(float_ty) => {
                let type_str = float_ty.print_to_string().to_string();
                if type_str.contains("float") {
                    Type::F32
                } else {
                    Type::F64
                }
            }
            BasicTypeEnum::PointerType(ptr_ty) => Type::Pointer(Box::new(Type::I8)),
            BasicTypeEnum::ArrayType(arr_ty) => {
                let len = arr_ty.len();
                let inner = Type::from_llvm_type(Some(arr_ty.get_element_type()));
                Type::Array(Box::new(inner), len)
            }
            _ => todo!("Implement alternative composite or struct types if needed"),
        }
    }
}

fn size_of_type(ty: &Type) -> u64 {
    match ty {
        Type::I8 | Type::U8 | Type::Bool | Type::Char => 1,
        Type::I16 | Type::U16 => 2,
        Type::I32 | Type::U32 | Type::F32 => 4,
        Type::I64 | Type::U64 | Type::F64 | Type::Pointer(_) => 8,
        Type::Array(inner_type, len) => size_of_type(inner_type) * (*len as u64),
        Type::Void => panic!("Compiler Error: cannot get size of void"),
    }
}

impl Expr {
    fn llvm_expr<'ctx>(
        &self,
        code_gen: &Codegen<'ctx>,
    ) -> Result<(BasicValueEnum<'ctx>, Type), String> {
        match self {
            Expr::SizeOf(ty) => {
                let u64_type = code_gen.context.i64_type();
                let const_val = u64_type.const_int(size_of_type(ty), false);
                Ok((const_val.into(), Type::U64))
            }
            Expr::Operand(x) => match x {
                Token::Int(t) => {
                    let i32_type = code_gen.context.i32_type();
                    let const_val = i32_type.const_int(t.clone() as u64, false);

                    return Ok((const_val.into(), Type::I32));
                }
                Token::Float(t) => {
                    let f64_type = code_gen.context.f64_type();
                    let const_val = f64_type.const_float(t.clone());

                    return Ok((const_val.into(), Type::F64));
                }
                Token::Bool(t) => {
                    let bool_type = code_gen.context.custom_width_int_type(1);
                    let const_val = bool_type.const_int(t.clone() as u64, false);

                    return Ok((const_val.into(), Type::Bool));
                }
                Token::Array(t) => {
                    if t.is_empty() {
                        return Err(
                            "Compiler Error: Cannot infer type of an empty array literal"
                                .to_string(),
                        );
                    }

                    let mut evaluated_constants = Vec::new();
                    let mut ty: Type = Type::I32;
                    for x in t.iter() {
                        let (val, typ) = x.llvm_expr(code_gen)?;
                        ty = typ;
                        evaluated_constants.push(val);
                    }

                    let element_type = evaluated_constants[0].get_type();

                    for (index, val) in evaluated_constants.iter().enumerate() {
                        if val.get_type() != element_type {
                            return Err(format!(
                                "Type Mismatch Error: Element at index {} does not match array element type {:?}",
                                index, element_type
                            ));
                        }
                    }

                    let const_array_val = match element_type {
                        inkwell::types::BasicTypeEnum::IntType(int_ty) => {
                            let int_values: Vec<inkwell::values::IntValue<'ctx>> =
                                evaluated_constants
                                    .iter()
                                    .map(|v| v.into_int_value())
                                    .collect();
                            int_ty.const_array(&int_values).into()
                        }
                        inkwell::types::BasicTypeEnum::FloatType(float_ty) => {
                            let float_values: Vec<inkwell::values::FloatValue<'ctx>> =
                                evaluated_constants
                                    .iter()
                                    .map(|v| v.into_float_value())
                                    .collect();
                            float_ty.const_array(&float_values).into()
                        }
                        _ => {
                            return Err(
                                "Error: Constant arrays of this type are not supported yet."
                                    .to_string(),
                            );
                        }
                    };

                    return Ok((const_array_val, Type::Array(Box::new(ty), t.len() as u32)));
                }
                Token::Identifier(name) => {
                    let table_ref = code_gen.symbol_table.borrow();

                    let (ptr, ty) = table_ref
                        .iter()
                        .rev()
                        .find_map(|scope| scope.get(name))
                        .cloned()
                        .ok_or_else(|| format!("Undefined variable: {}", name))?;

                    let llvm_type = ty.to_llvm_type(&code_gen);
                    let basic_type: BasicTypeEnum = llvm_type
                        .try_into()
                        .map_err(|_| format!("Cannot load variable '{}' of type {:?}", name, ty))?;

                    let value = code_gen
                        .builder
                        .build_load(basic_type, ptr, name)
                        .map_err(|e| e.to_string())?;
                    Ok((value, ty))
                }
                Token::Call(func_name, args_exprs) => {
                    if func_name == "array_get" {
                        return buildArrayGet(args_exprs, code_gen);
                    }
                    if func_name == "pop" {
                        return buildPop(args_exprs, code_gen);
                    }

                    let function = code_gen.module.get_function(func_name).expect(&format!(
                        "Compiler Error: Function {} is not defined.",
                        func_name
                    ));
                    let mut compiled_args: Vec<inkwell::values::BasicValueEnum<'ctx>> = Vec::new();
                    for arg_expr in args_exprs.iter() {
                        let (val, _ty) = arg_expr.llvm_expr(code_gen)?;
                        compiled_args.push(val);
                    }
                    let metadata_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> =
                        compiled_args.iter().map(|val| (*val).into()).collect();
                    let call_site = code_gen
                        .builder
                        .build_call(function, &metadata_args, &format!("{}_call_tmp", func_name))
                        .unwrap();

                    match call_site.try_as_basic_value().left() {
                        Some(basic_val) => {
                            let ret_ty = Type::from_llvm_type(Some(
                                function.get_type().get_return_type().unwrap(),
                            ));
                            Ok((basic_val, ret_ty))
                        }
                        None => Err(
                            "Compiler Error: Cannot use void function return as a value."
                                .to_string(),
                        ),
                    }
                }
                _ => Err(format!("not a valid Identifier {:?}", x)),
            },
            Expr::Operation(x, y) => {
                if y.len() == 2 {
                    if *x == Token::LBracket {
                        let (element_ptr, inner_ty) = buildIndexPointer(&y[0], &y[1], code_gen)?;
                        let inner_llvm_type: BasicTypeEnum = inner_ty
                            .to_llvm_type(code_gen)
                            .try_into()
                            .expect("Compiler Error: indexed pointer cannot point to void.");
                        let loaded_val = code_gen
                            .builder
                            .build_load(inner_llvm_type, element_ptr, "index_load")
                            .unwrap();
                        return Ok((loaded_val, inner_ty));
                    }

                    let (left_val, ty) = y[0].llvm_expr(code_gen)?;
                    let (right_val, tyR) = y[1].llvm_expr(code_gen)?;
                    match (left_val, right_val) {
                        (
                            inkwell::values::BasicValueEnum::IntValue(left_int),
                            inkwell::values::BasicValueEnum::IntValue(right_int),
                        ) => match x {
                            Token::Plus => {
                                let result = code_gen
                                    .builder
                                    .build_int_add(left_int, right_int, "add_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }
                            Token::Minus => {
                                let result = code_gen
                                    .builder
                                    .build_int_sub(left_int, right_int, "sub_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }
                            Token::Star => {
                                let result = code_gen
                                    .builder
                                    .build_int_mul(left_int, right_int, "mul_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }

                            Token::Slash => {
                                let result = if ty.is_signed() {
                                    code_gen
                                        .builder
                                        .build_int_signed_div(left_int, right_int, "sdiv_tmp")
                                        .unwrap()
                                } else {
                                    code_gen
                                        .builder
                                        .build_int_unsigned_div(left_int, right_int, "udiv_tmp")
                                        .unwrap()
                                };
                                return Ok((result.into(), ty));
                            }

                            Token::Greater => {
                                let pred = if ty.is_signed() {
                                    inkwell::IntPredicate::SGT
                                } else {
                                    inkwell::IntPredicate::UGT
                                };
                                let result = code_gen
                                    .builder
                                    .build_int_compare(pred, left_int, right_int, "cmp_gt_tmp")
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::Less => {
                                let pred = if ty.is_signed() {
                                    inkwell::IntPredicate::SLT
                                } else {
                                    inkwell::IntPredicate::ULT
                                };
                                let result = code_gen
                                    .builder
                                    .build_int_compare(pred, left_int, right_int, "cmp_lt_tmp")
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::EqualEqual => {
                                let result = code_gen
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        left_int,
                                        right_int,
                                        "cmp_eq_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::NotEqual => {
                                let result = code_gen
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::NE,
                                        left_int,
                                        right_int,
                                        "cmp_ne_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::GreaterEqual => {
                                let pred = if ty.is_signed() {
                                    inkwell::IntPredicate::SGE
                                } else {
                                    inkwell::IntPredicate::UGE
                                };
                                let result = code_gen
                                    .builder
                                    .build_int_compare(pred, left_int, right_int, "cmp_ge_tmp")
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::LessEqual => {
                                let pred = if ty.is_signed() {
                                    inkwell::IntPredicate::SLE
                                } else {
                                    inkwell::IntPredicate::ULE
                                };
                                let result = code_gen
                                    .builder
                                    .build_int_compare(pred, left_int, right_int, "cmp_le_tmp")
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }

                            _ => Err(format!("Unsupported integer operation: {:?}", x)),
                        },
                        (
                            inkwell::values::BasicValueEnum::FloatValue(left_float),
                            inkwell::values::BasicValueEnum::FloatValue(right_float),
                        ) => match x {
                            Token::Plus => {
                                let result = code_gen
                                    .builder
                                    .build_float_add(left_float, right_float, "fadd_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }
                            Token::Minus => {
                                let result = code_gen
                                    .builder
                                    .build_float_sub(left_float, right_float, "fsub_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }
                            Token::Star => {
                                let result = code_gen
                                    .builder
                                    .build_float_mul(left_float, right_float, "fmul_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }
                            Token::Slash => {
                                let result = code_gen
                                    .builder
                                    .build_float_div(left_float, right_float, "fdiv_tmp")
                                    .unwrap();
                                return Ok((result.into(), ty));
                            }
                            Token::Greater => {
                                let result = code_gen
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OGT,
                                        left_float,
                                        right_float,
                                        "fcmp_gt_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::Less => {
                                let result = code_gen
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OLT,
                                        left_float,
                                        right_float,
                                        "fcmp_lt_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::EqualEqual => {
                                let result = code_gen
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OEQ,
                                        left_float,
                                        right_float,
                                        "fcmp_eq_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::NotEqual => {
                                let result = code_gen
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::ONE,
                                        left_float,
                                        right_float,
                                        "fcmp_ne_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::GreaterEqual => {
                                let result = code_gen
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OGE,
                                        left_float,
                                        right_float,
                                        "fcmp_ge_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }
                            Token::LessEqual => {
                                let result = code_gen
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OLE,
                                        left_float,
                                        right_float,
                                        "fcmp_le_tmp",
                                    )
                                    .unwrap();
                                return Ok((result.into(), Type::Bool));
                            }

                            _ => todo!(),
                        },
                        _ => Err(format!(
                            "Type Mismatch Error: Cannot perform operation '{:?}' between mismatched types.",
                            x
                        )),
                    }
                } else if y.len() == 1 {
                    match x {
                        Token::LParen => {
                            return y
                                .get(0)
                                .expect("Compiler Error: Parenthesized expression is missing its inner expression.")
                                .llvm_expr(code_gen);
                        }
                        Token::Ampersand => {
                            // &expression
                            let expr = y.get(0).expect("& should have one operand");

                            // The operand needs to be something that has an address.
                            match expr {
                                Expr::Operand(Token::Identifier(name)) => {
                                    let (ptr, ty) = {
                                        let table_ref = code_gen.symbol_table.borrow();
                                        table_ref
                                            .iter()
                                            .rev()
                                            .find_map(|scope| scope.get(name))
                                            .cloned()
                                            .ok_or_else(|| {
                                                format!("Undefined variable: {}", name)
                                            })?
                                    };

                                    Ok((ptr.into(), Type::Pointer(Box::new(ty))))
                                }
                                _ => panic!(
                                    "Compiler Error: only named variables can be used with the address-of operator '&'."
                                ),
                            }
                        }
                        Token::Star => {
                            // *expression
                            let expr = y.get(0).expect("Dereference should have one operand");

                            let (evaluated_val, ty) = expr.llvm_expr(code_gen)?;

                            if let Type::Pointer(inner_ty) = ty {
                                let builder = &code_gen.builder;

                                let ptr_val = evaluated_val.into_pointer_value();

                                let llvm_load_type: inkwell::types::BasicTypeEnum = inner_ty
						            .to_llvm_type(code_gen)
						            .try_into()
						            .expect("Compiler Error: Cannot dereference a pointer to a void type.");

                                let loaded_val = builder
                                    .build_load(llvm_load_type, ptr_val, "deref_tmp")
                                    .unwrap();

                                Ok((loaded_val, *inner_ty))
                            } else {
                                Err(format!(
                                    "Compiler Error: Cannot dereference a non-pointer type: {:?}",
                                    ty
                                ))
                            }
                        }

                        _ => todo!(),
                    }
                } else {
                    return Err(format!(
                        "invalid amount of arguments: cannot perform the '{:?}' operation with '{:?}' arguments.",
                        x,
                        y.len()
                    ));
                }
            }
        }
    }
}

// #[derive(Debug, Clone,PartialEq)]
// pub enum Expr{
//     Operand(Token),
//     Operation(Token,Vec<Expr>),
// }

// Int(u32),
//   Float(f64),
//   Bool(bool),
//   String(String),
//   Array(Box<Vec<Expr>>),
//   Call(String,Box<Vec<Expr>>),

//   // identifiers
//   Identifier(String),
