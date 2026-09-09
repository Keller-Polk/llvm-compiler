use inkwell::*;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::GlobalValue;
use inkwell::types::{AnyTypeEnum, BasicTypeEnum};
use std::convert::TryInto;
use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::FunctionValue;
use std::collections::HashMap;
use inkwell::values::PointerValue;
use inkwell::values::{BasicValue, InstructionValue}; 


use crate::lexer::*;
use crate::parser::*;


pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub symbol_table: std::cell::RefCell<HashMap<String, (PointerValue<'ctx>, Type)>>,
}
pub enum Components<'ctx> {
	Function(FunctionValue<'ctx>),
	Global(GlobalValue<'ctx>)
}



impl Program {
	pub fn build(& self){
		// init
		let context = Context::create();
	    let module = context.create_module("my_compiler");
	    let builder = context.create_builder();

	    let code_gen = Codegen{context: &context,module:module,builder:builder,symbol_table: std::cell::RefCell::new(HashMap::new())};





		// 1. Create a hidden dummy function frame
		let void_type = context.void_type();
		let dummy_fn_type = void_type.fn_type(&[], false);
		let dummy_func = code_gen.module.add_function("__global_init_sandbox", dummy_fn_type, None);

		// 2. Append an empty basic block and force position the cursor inside it
		let sandbox_block = context.append_basic_block(dummy_func, "sandbox");
		code_gen.builder.position_at_end(sandbox_block);
		 

    	///  

    	for x in self.items.clone() {
    		let item = match x {
    			Item::Global(global_declaration) => Ok(Components::Global(global_declaration.buildGlobal(&code_gen))),
    			Item::Function(func_def) => Ok(Components::Function(func_def.buildFunction(&code_gen).expect("your function has an error"))),
    			_ => Err("not a function or a gloabal defintion"),
    		};
    	}

    	unsafe {
		    dummy_func.delete();
		}

    	code_gen.module.verify().unwrap();
    	println!("--- Generated LLVM IR ---");
    	code_gen.module.print_to_stderr();
    }
}

// #[derive(Debug, Clone,PartialEq)]
// pub struct FunctionDecl {
//     pub name: String,
//     pub params: Vec<Param>,
//     pub return_type: Type,
//     pub body: Vec<Stmt>,
// }

impl FunctionDecl{
    fn buildFunction<'ctx>(& self, code_gen: &Codegen<'ctx>) -> Result<FunctionValue<'ctx>, String>{
    	let context = &code_gen.context;
	    let module =  &code_gen.module;
	    let builder = &code_gen.builder;

		let mut arg_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
		for param in &self.params {
		    let llvm_type = param.ty.to_llvm_type(&code_gen); 
		    let basic_type: inkwell::types::BasicTypeEnum = llvm_type.try_into().map_err(|_| "Void is not allowed as a parameter type.")?;
		        
		    arg_types.push(BasicMetadataTypeEnum::from(basic_type));
		}
	    let return_type = self.return_type.to_llvm_type(&code_gen);

		let fn_type = match return_type {
		    inkwell::types::AnyTypeEnum::VoidType(void_ty) => void_ty.fn_type(&arg_types, false),
		    inkwell::types::AnyTypeEnum::IntType(int_ty) => int_ty.fn_type(&arg_types, false),
		    inkwell::types::AnyTypeEnum::FloatType(float_ty) => float_ty.fn_type(&arg_types, false),
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
		    code_gen.symbol_table.borrow_mut().insert(param_name.clone(), (alloca,param_type));
		}

		println!("{:?}",code_gen.symbol_table);
		for x in self.body.clone(){
			match x {
				Stmt::Assignment(x) => x.buildAssignment(&code_gen),
				Stmt::Return(x) => buildReturn(&x,&code_gen),
				Stmt::Expr(x) =>  x.buildExprs(&code_gen), 
				_ => todo!(),
			};
		}

		let current_block = code_gen.builder.get_insert_block().unwrap();
		if current_block.get_terminator().is_none() {
		    let return_type = self.return_type.to_llvm_type(&code_gen);
		    
		    match return_type {
		        inkwell::types::AnyTypeEnum::VoidType(_) => {
		            code_gen.builder.build_return(None).unwrap();
		        },
		        inkwell::types::AnyTypeEnum::IntType(int_ty) => {
		            let default_zero = int_ty.const_int(0, false);
		            code_gen.builder.build_return(Some(&default_zero)).unwrap();
		        },
		        inkwell::types::AnyTypeEnum::FloatType(float_ty) => {
		            let default_zero = float_ty.const_float(0.0);
		            code_gen.builder.build_return(Some(&default_zero)).unwrap();
		        },
		        _ => return Err("Missing explicit return statement for this return type.".to_string()),
		    };
		}
	   	Ok(function)
    }
}

impl AssignmentStmt {
	fn buildAssignment<'ctx>(&self, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
	    let context = &code_gen.context;
	    let module = &code_gen.module;
	    let builder = &code_gen.builder;

	    let llvm_any_type = match self.ty.clone(){
	    	Some(x) => x.to_llvm_type(&code_gen),
	    	None => {
	    		let (_, ty) = code_gen.symbol_table.borrow().get(&self.name).cloned().expect(&format!("{:?} has not been defined.",&self.name));
	    		ty.to_llvm_type(&code_gen)
	    	}
	    };
	    

	    let llvm_basic_type: inkwell::types::BasicTypeEnum = llvm_any_type
	        .try_into()
	        .expect("Compiler Error: Local variables cannot be initialized with Void types.");

	    let local_var = builder.build_alloca(llvm_basic_type, &self.name).unwrap();
	    println!("{:?} safdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",self.value.clone());

	    let (evaluated_val, ty) = self.value.clone()
	        .expect("Compiler Error: Local variables must be initialized with a default value.")
	        .llvm_expr(code_gen)
	        .expect("REASON");


		let store_instruction = builder.build_store(local_var, evaluated_val).unwrap();

		if !code_gen.symbol_table.borrow().contains_key(&self.name){
	    	code_gen.symbol_table.borrow_mut().insert(self.name.clone(), (local_var,ty));
	    }

		store_instruction
	}

}
impl Expr {
	fn buildExprs<'ctx>(&self, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
		let (evaluated_val, _ty) = self
            .llvm_expr(code_gen)
            .expect("Compiler Error: Failed to codegen standalone expression line.");

        evaluated_val
            .as_instruction_value()
            .expect("Compiler Error: Standalone expression did not produce an instruction.")

	}
}

fn buildReturn<'ctx>(expr: &Option<Expr>, code_gen: &Codegen<'ctx>) -> InstructionValue<'ctx> {
    let builder = &code_gen.builder;

    if expr.is_none() {
        builder.build_return(None).unwrap()
    } else {
        let (local_var, _ty) = expr.clone()
            .expect("Compiler Error: Local variables must be initialized with a default value.")
            .llvm_expr(code_gen)
            .expect("Failed to codegen return expression");

        builder.build_return(Some(&local_var)).unwrap()
    }
}


impl GlobalDecl{
    fn buildGlobal<'ctx>(& self, code_gen: &Codegen<'ctx>) -> GlobalValue<'ctx>  {
    	let context = &code_gen.context;
	    let module =  &code_gen.module;
	    let builder = &code_gen.builder;

	    let llvm_any_type = self.ty.to_llvm_type(code_gen);

    	let llvm_basic_type: inkwell::types::BasicTypeEnum = llvm_any_type
	    .try_into()
	    .expect("Compiler Error: Global variables cannot be initialized with Void types.");

		    
		    
		let global_var = module.add_global(llvm_basic_type, None, &self.name);
		global_var.set_linkage(inkwell::module::Linkage::External);


		let (evaluated_val,ty) = self.value.clone()
		    .expect("global varbles cannot be initialized without declareing a defalt value for it as they are constant")
		    .llvm_expr(code_gen)
		    .expect("REASON");

		let default_initializer: inkwell::values::BasicValueEnum<'ctx> = if self.pointer {
		    if evaluated_val.is_pointer_value() {
		        evaluated_val
		    } else if evaluated_val.is_int_value() {
		        let int_val = evaluated_val.into_int_value();
		        let ptr_type = code_gen.context.ptr_type(inkwell::AddressSpace::from(0));
		        
		        int_val.const_to_pointer(ptr_type).into()
		    } else {
		        panic!("Compiler Error: Cannot initialize a global pointer with a floating-point value.");
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

impl Type{
    pub fn is_signed(&self) -> bool {
        match self {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 => true,
            _ => false, // All U variants, Bool, Char, and Arrays evaluate to false
        }
    }

	fn to_llvm_type<'ctx>(& self, code_gen: &Codegen<'ctx>) -> AnyTypeEnum<'ctx>{
		let context = &code_gen.context;
	    let module =  &code_gen.module;
	    let builder = &code_gen.builder;
		match self {
            Type::I8  => context.i8_type().into(),
            Type::I16 => context.i16_type().into(),
            Type::I32 => context.i32_type().into(),
            Type::I64 => context.i64_type().into(),

            Type::U8  => context.i8_type().into(),
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
            },
			Type::Pointer(inner_type) => {
			    let any_inner = inner_type.to_llvm_type(code_gen);

			    let basic_inner: BasicTypeEnum = any_inner
			        .try_into()
			        .expect("Compiler Error: Cannot create a pointer to Void.");

			    basic_inner.ptr_type(inkwell::AddressSpace::default()).into()
			}

        }
	}
	fn from_llvm_type(opt_llvm_ty: Option<BasicTypeEnum>) -> Self {
        let llvm_ty = match opt_llvm_ty {
            None => return Type::Void, 
            Some(ty) => ty,
        };

        match llvm_ty {
            BasicTypeEnum::IntType(int_ty) => {
                match int_ty.get_bit_width() {
                    1 => Type::Bool,
                    8 => Type::I8,   
                    16 => Type::I16,
                    32 => Type::I32,
                    64 => Type::I64,
                    _ => panic!("Unsupported LLVM integer bit-width"),
                }
            }
            BasicTypeEnum::FloatType(float_ty) => {
                let type_str = float_ty.print_to_string().to_string();
                if type_str.contains("float") {
                    Type::F32
                } else {
                    Type::F64
                }
            }
            BasicTypeEnum::PointerType(ptr_ty) => {
                Type::Pointer(Box::new(Type::I8))
            }
            BasicTypeEnum::ArrayType(arr_ty) => {
                let len = arr_ty.len();
                let inner = Type::from_llvm_type(Some(arr_ty.get_element_type()));
                Type::Array(Box::new(inner), len)
            }
            _ => todo!("Implement alternative composite or struct types if needed"),
        }
    }
}



impl Expr{
	fn llvm_expr<'ctx>(& self, code_gen: &Codegen<'ctx>) -> Result<(BasicValueEnum<'ctx>,Type), String>{
		println!("{:?}fff",self);
		match self{
			Expr::Operand(x) =>
			{
				match x {
					Token::Int(t)=>{
						let i32_type = code_gen.context.i32_type();
						let const_val = i32_type.const_int(t.clone() as u64, false);

						return Ok((const_val.into(),Type::I32));
					},
					Token::Float(t)=>{
						let f64_type = code_gen.context.f64_type();
						let const_val = f64_type.const_float(t.clone());

						return Ok((const_val.into(),Type::F64));
					},
					Token::Bool(t)=>{
						let bool_type = code_gen.context.custom_width_int_type(1);
						let const_val = bool_type.const_int(t.clone() as u64,false);

						return Ok((const_val.into(),Type::Bool));
					},
					Token::Array(t) => {
					    if t.is_empty() {
					        return Err("Compiler Error: Cannot infer type of an empty array literal".to_string());
					    }

					    let mut evaluated_constants = Vec::new();
					    let mut ty:Type = Type::I32;
					    for x in t.iter() {
					        let (val,typ) = x.llvm_expr(code_gen)?;
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
					            let int_values: Vec<inkwell::values::IntValue<'ctx>> = evaluated_constants
					                .iter()
					                .map(|v| v.into_int_value())
					                .collect();
					            int_ty.const_array(&int_values).into()
					        }
					        inkwell::types::BasicTypeEnum::FloatType(float_ty) => {
					            let float_values: Vec<inkwell::values::FloatValue<'ctx>> = evaluated_constants
					                .iter()
					                .map(|v| v.into_float_value())
					                .collect();
					            float_ty.const_array(&float_values).into()
					        }
					        _ => {
					            return Err("Error: Constant arrays of this type are not supported yet.".to_string());
					        }
					    };

					    return Ok((const_array_val,Type::Array(Box::new(ty),t.len() as u32)));
					},
					Token::Identifier(name) => {
    					let (ptr, ty) = code_gen.symbol_table.borrow().get(name).cloned().ok_or_else(|| format!("Undefined variable: {}", name))?;

					    let llvm_type = ty.to_llvm_type(&code_gen);

						let basic_type: BasicTypeEnum = llvm_type.try_into().map_err(|_| format!("Cannot load variable '{}' of type {:?}", name, ty))?;

						let value = code_gen.builder.build_load(basic_type, ptr, name).map_err(|e| e.to_string())?;

					    Ok((value, ty))
					},
					Token::Call(func_name,args_exprs) =>{
						let function = code_gen.module.get_function(func_name)
        					.expect(&format!("Compiler Error: Function {} is not defined.", func_name));
        				let mut compiled_args: Vec<inkwell::values::BasicValueEnum<'ctx>> = Vec::new();
						for arg_expr in args_exprs.iter() {
					        let (val, _ty) = arg_expr.llvm_expr(code_gen)?;
					        compiled_args.push(val);
								    }
					    let metadata_args: Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> = compiled_args
					        .iter()
					        .map(|val| (*val).into())
					        .collect();
						let call_site = code_gen.builder
							.build_call(function, &metadata_args, &format!("{}_call_tmp", func_name))
					        .unwrap();

						match call_site.try_as_basic_value().left() {
							Some(basic_val) => {
					            let ret_ty = Type::from_llvm_type(Some(function.get_type().get_return_type().unwrap())); 
					            Ok((basic_val, ret_ty))
					        }
					        None => {
					            panic!("Handling for void function returns goes here");
					        }
					    }
					},	
					_ => {
						panic!("not a valid Identifier {:?}",x);
					},

				}
			},
			Expr::Operation(x,y)=>
			{
				if y.len() == 2{
					let (left_val,ty) = y[0].llvm_expr(code_gen)?;
					let (right_val,tyR) = y[1].llvm_expr(code_gen)?;
					match (left_val, right_val) { 	
						(inkwell::values::BasicValueEnum::IntValue(left_int), inkwell::values::BasicValueEnum::IntValue(right_int)) => {
							match x{
								Token::Plus =>{
									let result = code_gen.builder.build_int_add(left_int, right_int, "add_tmp").unwrap();
                					return Ok((result.into(),ty))
								},
								Token::Minus =>{
									let result = code_gen.builder.build_int_sub(left_int, right_int, "sub_tmp").unwrap();
                					return Ok((result.into(),ty))
								},
								Token::Star =>{
									let result = code_gen.builder.build_int_mul(left_int, right_int, "mul_tmp").unwrap();
                					return Ok((result.into(),ty))
								},

								Token::Slash => {
								    let result = if ty.is_signed() {
								        code_gen.builder.build_int_signed_div(left_int, right_int, "sdiv_tmp").unwrap()
								    } else {
								        code_gen.builder.build_int_unsigned_div(left_int, right_int, "udiv_tmp").unwrap()
								    };
								    return Ok((result.into(), ty))
								},

								Token::Greater => {
				                    let pred = if ty.is_signed() { inkwell::IntPredicate::SGT } else { inkwell::IntPredicate::UGT };
				                    let result = code_gen.builder.build_int_compare(pred, left_int, right_int, "cmp_gt_tmp").unwrap();
				                    return Ok((result.into(), Type::Bool))
				                },
				                Token::Less => {
				                    let pred = if ty.is_signed() { inkwell::IntPredicate::SLT } else { inkwell::IntPredicate::ULT };
				                    let result = code_gen.builder.build_int_compare(pred, left_int, right_int, "cmp_lt_tmp").unwrap();
				                    return Ok((result.into(), Type::Bool))
				                },
				                Token::EqualEqual => {
				                    let result = code_gen.builder.build_int_compare(inkwell::IntPredicate::EQ, left_int, right_int, "cmp_eq_tmp").unwrap();
				                    return Ok((result.into(), Type::Bool))
				                },
				                Token::GreaterEqual => {
								    let pred = if ty.is_signed() { inkwell::IntPredicate::SGE } else { inkwell::IntPredicate::UGE };
								    let result = code_gen.builder.build_int_compare(pred, left_int, right_int, "cmp_ge_tmp").unwrap();
								    return Ok((result.into(), Type::Bool))
								},
								Token::LessEqual => {
								    let pred = if ty.is_signed() { inkwell::IntPredicate::SLE } else { inkwell::IntPredicate::ULE };
								    let result = code_gen.builder.build_int_compare(pred, left_int, right_int, "cmp_le_tmp").unwrap();
								    return Ok((result.into(), Type::Bool))
								},
							
								_ => {
									panic!("{:?}",x);
								}
							}
						}
						(inkwell::values::BasicValueEnum::FloatValue(left_float), inkwell::values::BasicValueEnum::FloatValue(right_float)) => {
							match x{
								Token::Plus =>{
									let result = code_gen.builder.build_float_add(left_float, right_float, "fadd_tmp").unwrap();
                					return Ok((result.into(),ty))
								},
								Token::Minus =>{
									let result = code_gen.builder.build_float_sub(left_float, right_float, "fsub_tmp").unwrap();
                					return Ok((result.into(),ty))
								},
								Token::Star =>{
									let result = code_gen.builder.build_float_mul(left_float, right_float, "fmul_tmp").unwrap();
                					return Ok((result.into(),ty))
								},
								Token::Slash => {
								    let result = code_gen.builder.build_float_div(left_float, right_float, "fdiv_tmp").unwrap();
								    return Ok((result.into(), ty))
								},
								Token::Greater => {
				                    let result = code_gen.builder.build_float_compare(inkwell::FloatPredicate::OGT, left_float, right_float, "fcmp_gt_tmp").unwrap();
				                    return Ok((result.into(), Type::Bool))
				                },
				                Token::Less => {
				                    let result = code_gen.builder.build_float_compare(inkwell::FloatPredicate::OLT, left_float, right_float, "fcmp_lt_tmp").unwrap();
				                    return Ok((result.into(), Type::Bool))
				                },
				                Token::EqualEqual => {
				                    let result = code_gen.builder.build_float_compare(inkwell::FloatPredicate::OEQ, left_float, right_float, "fcmp_eq_tmp").unwrap();
				                    return Ok((result.into(), Type::Bool))
				                },
								Token::GreaterEqual => {
								    let result = code_gen.builder.build_float_compare(inkwell::FloatPredicate::OGE, left_float, right_float, "fcmp_ge_tmp").unwrap();
								    return Ok((result.into(), Type::Bool))
								},
								Token::LessEqual => {
								    let result = code_gen.builder.build_float_compare(inkwell::FloatPredicate::OLE, left_float, right_float, "fcmp_le_tmp").unwrap();
								    return Ok((result.into(), Type::Bool))
								},

								_ => todo!()
							}
						}
						_ => {
					        Err(format!(
					            "Type Mismatch Error: Cannot perform operation '{:?}' between mismatched types.",
					            x
					        ))
					    }
					}
				}else if y.len() == 1{
					match x{
						Token::LParen => {
							return y.get(0).expect("this should have one argument").llvm_expr(code_gen);
						},
						Token::Ampersand => {
				            // &expression
				            let expr = y
				                .get(0)
				                .expect("& should have one operand");

				            // The operand needs to be something that has an address.
				            match expr {
				                Expr::Operand(Token::Identifier(name)) => {
				                    let (ptr, ty) = code_gen
				                        .symbol_table
				                        .borrow()
				                        .get(name)
				                        .cloned()
				                        .ok_or_else(|| {
				                            format!("Undefined variable: {}", name)
				                        })?;

				                    let ty = code_gen
				                        .symbol_table
				                        .borrow()
				                        .get(name)
				                        .map(|(_, ty)| ty.clone())
				                        .ok_or_else(|| {
				                            format!("Undefined variable: {}", name)
				                        })?;

				                    Ok((ptr.into(), Type::Pointer(Box::new(ty))))
				                }
				                _ => panic!("operations cannot be addressed"),
				            }
				        },
				        Token::Star => {
						    // *expression
						    let expr = y
						        .get(0)
						        .expect("Dereference should have one operand");

						    let (evaluated_val, ty) = expr.llvm_expr(code_gen)?;

						    if let Type::Pointer(inner_ty) = ty {
						        let builder = &code_gen.builder;

						        let ptr_val = evaluated_val.into_pointer_value();

						        let llvm_load_type: inkwell::types::BasicTypeEnum = inner_ty
						            .to_llvm_type(code_gen)
						            .try_into()
						            .expect("Compiler Error: Cannot dereference a pointer to a void type.");

						        let loaded_val = builder.build_load(llvm_load_type, ptr_val, "deref_tmp").unwrap();

						        Ok((loaded_val, *inner_ty))
						    } else {
						        panic!("Compiler Error: Cannot dereference a non-pointer type: {:?}", ty);
						    }
						},

				        _ => todo!(),
					}
				}else{
					return Err(format!(
					    "invalid amount of arguments: cannot perform the '{:?}' operation with '{:?}' arguments.",
					    x,
				        y.len()
			        ))
				}

			},

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
