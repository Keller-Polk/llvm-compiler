use compiler::lexer::*;
use compiler::parser::*;
use std::process::Command;
use std::sync::Mutex;

static BUILD_LOCK: Mutex<()> = Mutex::new(());

fn parse_program(input: &str) -> Program {
    let tokens = Token::lexer(input);
    let mut parser = TokenParse::convert_from_lex(tokens);
    parser.parse()
}

fn assert_parse_ok(input: &str) {
    let program = parse_program(input);
    let _ = format!("{:?}", program);
}

fn assert_build_ok(input: &str) {
    let _guard = BUILD_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let program = parse_program(input);
    program.build("output");
}

fn assert_build_runs_with_output(input: &str, expected_stdout: &str) {
    let _guard = BUILD_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let program = parse_program(input);
    program.build("output");

    let output = Command::new("./output")
        .output()
        .expect("generated executable should run");

    assert!(
        output.status.success(),
        "generated executable failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(String::from_utf8_lossy(&output.stdout), expected_stdout);
}

macro_rules! parse_ok_tests {
    ($($name:ident => $source:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_parse_ok($source);
            }
        )*
    };
}

macro_rules! build_ok_tests {
    ($($name:ident => $source:expr;)*) => {
        $(
            #[test]
            fn $name() {
                assert_build_ok($source);
            }
        )*
    };
}

macro_rules! run_output_tests {
    ($($name:ident => ($source:expr, $expected:expr);)*) => {
        $(
            #[test]
            fn $name() {
                assert_build_runs_with_output($source, $expected);
            }
        )*
    };
}

parse_ok_tests! {
    parse_global_i8 => "i8: x = 1;";
    parse_global_i16 => "i16: x = 1;";
    parse_global_i32 => "i32: x = 1;";
    parse_global_i64 => "i64: x = 1;";
    parse_global_u8 => "u8: x = 1;";
    parse_global_u16 => "u16: x = 1;";
    parse_global_u32 => "u32: x = 1;";
    parse_global_u64 => "u64: x = 1;";
    parse_global_bool_true => "bool: x = true;";
    parse_global_bool_false => "bool: x = false;";
    parse_global_float_f32 => "f32: x = 1.5;";
    parse_global_float_f64 => "f64: x = 2.5;";
    parse_pointer_decl => "i32: *ptr;";
    parse_pointer_init_malloc => "fn void run(){ i32: *ptr = malloc(sizeof(i32)); }";
    parse_local_assignment_let => "fn void run(){ let i32: x = 10; }";
    parse_local_assignment_no_let => "fn void run(){ i32: x = 10; }";
    parse_short_reassignment => "fn void run(){ i32: x = 1; x = 2; }";
    parse_add_expr => "i32: x = 1 + 2;";
    parse_sub_expr => "i32: x = 3 - 2;";
    parse_mul_expr => "i32: x = 3 * 2;";
    parse_div_expr => "i32: x = 6 / 2;";
    parse_precedence_expr => "i32: x = 1 + 2 * 3;";
    parse_grouped_expr => "i32: x = (1 + 2) * 3;";
    parse_equal_expr => "bool: x = 1 == 1;";
    parse_not_equal_expr => "bool: x = 1 != 2;";
    parse_less_expr => "bool: x = 1 < 2;";
    parse_less_equal_expr => "bool: x = 1 <= 2;";
    parse_greater_expr => "bool: x = 2 > 1;";
    parse_greater_equal_expr => "bool: x = 2 >= 1;";
    parse_function_empty_void => "fn void run(){ }";
    parse_function_empty_i32 => "fn i32 main(){ }";
    parse_function_return_i32 => "fn i32 main(){ return 1; }";
    parse_function_return_void => "fn void run(){ return; }";
    parse_function_one_param => "fn i32 id(x:i32){ return x; }";
    parse_function_two_params_comma => "fn i32 add(a:i32,b:i32){ return a + b; }";
    parse_function_two_params_space => "fn i32 add(a:i32 b:i32){ return a + b; }";
    parse_function_call_empty => "fn void run(){ foo(); }";
    parse_function_call_one_arg => "fn void run(){ foo(1); }";
    parse_function_call_two_args => "fn void run(){ foo(1, 2); }";
    parse_nested_function_call => "fn void run(){ print(add(1, 2)); }";
    parse_if_true => "fn void run(){ if true { return; } }";
    parse_if_comparison => "fn void run(){ if (1 == 1) { return; } }";
    parse_if_else => "fn void run(){ if false { return; } else { return; } }";
    parse_while_false => "fn void run(){ while false { break; } }";
    parse_while_continue => "fn void run(){ while false { continue; } }";
    parse_while_nested_if => "fn void run(){ while false { if true { break; } } }";
    parse_break => "fn void run(){ while true { break; } }";
    parse_continue => "fn void run(){ while false { continue; } }";
    parse_sizeof_i8 => "u64: x = sizeof(i8);";
    parse_sizeof_i16 => "u64: x = sizeof(i16);";
    parse_sizeof_i32 => "u64: x = sizeof(i32);";
    parse_sizeof_i64 => "u64: x = sizeof(i64);";
    parse_sizeof_u8 => "u64: x = sizeof(u8);";
    parse_sizeof_u16 => "u64: x = sizeof(u16);";
    parse_sizeof_u32 => "u64: x = sizeof(u32);";
    parse_sizeof_u64 => "u64: x = sizeof(u64);";
    parse_sizeof_bool => "u64: x = sizeof(bool);";
    parse_sizeof_char => "u64: x = sizeof(char);";
    parse_sizeof_array => "u64: x = sizeof(i32[3]);";
    parse_sizeof_in_call => "fn void run(){ print(sizeof(i32)); }";
    parse_array_literal_i32 => "i32[3]: nums = [1,2,3];";
    parse_array_literal_bool => "bool[2]: flags = [true,false];";
    parse_index_assignment => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); arr[0] = 1; }";
    parse_index_expression => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); print(arr[0]); }";
    parse_array_get => "fn void run(){ print(array_get(arr, 0)); }";
    parse_array_set => "fn void run(){ array_set(arr, 0, 1); }";
    parse_push => "fn void run(){ push(arr, &len, 1); }";
    parse_pop => "fn void run(){ print(pop(arr, &len)); }";
    parse_address_of => "fn void run(){ i32: x = 1; i32: *p = &x; }";
    parse_deref_expr => "fn void run(){ i32: x = 1; i32: *p = &x; print(*p); }";
    parse_deref_assignment => "fn void run(){ i32: x = 1; i32: *p = &x; *p = 2; }";
    parse_malloc_call => "fn void run(){ i32: *p = malloc(sizeof(i32)); }";
    parse_free_call => "fn void run(){ i32: *p = malloc(sizeof(i32)); free(p); }";
    parse_print_int => "fn void run(){ print(1); }";
    parse_print_bool => "fn void run(){ print(true); }";
    parse_print_pointer => "fn void run(){ i32: *p = malloc(sizeof(i32)); print(p); }";
    parse_pointer_return_function => "fn *i32 make(){ i32: *p = malloc(sizeof(i32)); return p; }";
}

build_ok_tests! {
    build_empty_run => "fn void run(){ }";
    build_i32_main => "fn i32 main(){ return 0; }";
    build_global_i32_and_run => "i32: x = 1; fn void run(){ }";
    build_global_bool_and_run => "bool: flag = true; fn void run(){ }";
    build_local_i8 => "fn void run(){ i8: x = 1; }";
    build_local_i16 => "fn void run(){ i16: x = 1; }";
    build_local_i32 => "fn void run(){ i32: x = 1; }";
    build_local_i64 => "fn void run(){ i64: x = 1; }";
    build_local_u8 => "fn void run(){ u8: x = 1; }";
    build_local_u16 => "fn void run(){ u16: x = 1; }";
    build_local_u32 => "fn void run(){ u32: x = 1; }";
    build_local_u64 => "fn void run(){ u64: x = 1; }";
    build_local_bool => "fn void run(){ bool: x = true; }";
    build_add => "fn void run(){ i32: x = 1 + 2; }";
    build_sub => "fn void run(){ i32: x = 3 - 2; }";
    build_mul => "fn void run(){ i32: x = 3 * 2; }";
    build_div => "fn void run(){ i32: x = 6 / 2; }";
    build_comparison_eq => "fn void run(){ bool: x = 1 == 1; }";
    build_comparison_ne => "fn void run(){ bool: x = 1 != 2; }";
    build_comparison_lt => "fn void run(){ bool: x = 1 < 2; }";
    build_comparison_le => "fn void run(){ bool: x = 1 <= 2; }";
    build_comparison_gt => "fn void run(){ bool: x = 2 > 1; }";
    build_comparison_ge => "fn void run(){ bool: x = 2 >= 1; }";
    build_return_i32 => "fn i32 main(){ return 123; }";
    build_return_from_helper => "fn i32 one(){ return 1; } fn void run(){ i32: x = one(); }";
    build_function_params => "fn i32 add(a:i32,b:i32){ return a + b; } fn void run(){ i32: x = add(1, 2); }";
    build_if_true => "fn void run(){ if true { i32: x = 1; } }";
    build_if_false_else => "fn void run(){ if false { i32: x = 1; } else { i32: y = 2; } }";
    build_while_false => "fn void run(){ while false { i32: x = 1; } }";
    build_while_break => "fn void run(){ while true { break; } }";
    build_while_continue_unreached => "fn void run(){ while false { continue; } }";
    build_sizeof_i32 => "fn void run(){ u64: x = sizeof(i32); }";
    build_sizeof_array => "fn void run(){ u64: x = sizeof(i32[4]); }";
    build_print_int => "fn void run(){ print(1); }";
    build_print_bool => "fn void run(){ print(true); }";
    build_print_i64 => "fn void run(){ i64: x = 1; print(x); }";
    build_print_u64 => "fn void run(){ u64: x = 1; print(x); }";
    build_pointer_address_of => "fn void run(){ i32: x = 1; i32: *p = &x; }";
    build_pointer_deref_load => "fn void run(){ i32: x = 1; i32: *p = &x; i32: y = *p; }";
    build_pointer_deref_store => "fn void run(){ i32: x = 1; i32: *p = &x; *p = 2; }";
    build_malloc_free_i32 => "fn void run(){ i32: *p = malloc(sizeof(i32)); free(p); }";
    build_malloc_free_u64 => "fn void run(){ u64: *p = malloc(sizeof(u64)); free(p); }";
    build_index_set => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); arr[0] = 1; free(arr); }";
    build_index_get => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); arr[0] = 1; i32: x = arr[0]; free(arr); }";
    build_array_set_builtin => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); array_set(arr, 0, 1); free(arr); }";
    build_array_get_builtin => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); array_set(arr, 0, 1); i32: x = array_get(arr, 0); free(arr); }";
    build_push_builtin => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); i32: len = 0; push(arr, &len, 1); free(arr); }";
    build_pop_builtin => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); i32: len = 0; push(arr, &len, 1); i32: x = pop(arr, &len); free(arr); }";
    build_two_pushes_two_pops => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 4); i32: len = 0; push(arr, &len, 1); push(arr, &len, 2); i32: a = pop(arr, &len); i32: b = pop(arr, &len); free(arr); }";
    build_array_index_with_variable => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); i32: i = 1; arr[i] = 7; i32: x = arr[i]; free(arr); }";
    build_nested_array_get_print => "fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); array_set(arr, 0, 7); print(array_get(arr, 0)); free(arr); }";
    build_pointer_return_explicit => "fn *i32 make(){ i32: *p = malloc(sizeof(i32)); return p; } fn void run(){ i32: *p = make(); free(p); }";
    build_pointer_return_default_null => "fn *i32 make(){ } fn void run(){ i32: *p = make(); }";
}

run_output_tests! {
    run_print_int => ("fn void run(){ print(1); }", "1\n");
    run_print_bool_true => ("fn void run(){ print(true); }", "1\n");
    run_print_bool_false => ("fn void run(){ print(false); }", "0\n");
    run_print_add => ("fn void run(){ print(1 + 2); }", "3\n");
    run_print_sub => ("fn void run(){ print(5 - 2); }", "3\n");
    run_print_mul => ("fn void run(){ print(3 * 4); }", "12\n");
    run_print_div => ("fn void run(){ print(8 / 2); }", "4\n");
    run_print_precedence => ("fn void run(){ print(1 + 2 * 3); }", "7\n");
    run_print_grouping => ("fn void run(){ print((1 + 2) * 3); }", "9\n");
    run_print_equal_true => ("fn void run(){ print(1 == 1); }", "1\n");
    run_print_equal_false => ("fn void run(){ print(1 == 2); }", "0\n");
    run_print_not_equal_true => ("fn void run(){ print(1 != 2); }", "1\n");
    run_print_less_true => ("fn void run(){ print(1 < 2); }", "1\n");
    run_print_less_equal_true => ("fn void run(){ print(2 <= 2); }", "1\n");
    run_print_greater_true => ("fn void run(){ print(3 > 2); }", "1\n");
    run_print_greater_equal_true => ("fn void run(){ print(3 >= 3); }", "1\n");
    run_print_local_var => ("fn void run(){ i32: x = 42; print(x); }", "42\n");
    run_print_i64 => ("fn void run(){ i64: x = 42; print(x); }", "42\n");
    run_print_u64 => ("fn void run(){ u64: x = 42; print(x); }", "42\n");
    run_function_return => ("fn i32 get(){ return 55; } fn void run(){ print(get()); }", "55\n");
    run_function_add_params => ("fn i32 add(a:i32,b:i32){ return a + b; } fn void run(){ print(add(20, 22)); }", "42\n");
    run_if_true_prints_then => ("fn void run(){ if true { print(7); } }", "7\n");
    run_if_false_prints_else => ("fn void run(){ if false { print(1); } else { print(2); } }", "2\n");
    run_while_false_skips_body => ("fn void run(){ while false { print(99); } print(1); }", "1\n");
    run_while_true_breaks => ("fn void run(){ while true { print(1); break; print(2); } print(3); }", "1\n3\n");
    run_sizeof_i8 => ("fn void run(){ print(sizeof(i8)); }", "1\n");
    run_sizeof_i16 => ("fn void run(){ print(sizeof(i16)); }", "2\n");
    run_sizeof_i32 => ("fn void run(){ print(sizeof(i32)); }", "4\n");
    run_sizeof_i64 => ("fn void run(){ print(sizeof(i64)); }", "8\n");
    run_sizeof_u8 => ("fn void run(){ print(sizeof(u8)); }", "1\n");
    run_sizeof_u16 => ("fn void run(){ print(sizeof(u16)); }", "2\n");
    run_sizeof_u32 => ("fn void run(){ print(sizeof(u32)); }", "4\n");
    run_sizeof_u64 => ("fn void run(){ print(sizeof(u64)); }", "8\n");
    run_sizeof_bool => ("fn void run(){ print(sizeof(bool)); }", "1\n");
    run_sizeof_char => ("fn void run(){ print(sizeof(char)); }", "1\n");
    run_sizeof_i32_array_three => ("fn void run(){ print(sizeof(i32[3])); }", "12\n");
    run_pointer_deref_load => ("fn void run(){ i32: x = 77; i32: *p = &x; print(*p); }", "77\n");
    run_pointer_deref_store => ("fn void run(){ i32: x = 1; i32: *p = &x; *p = 88; print(x); }", "88\n");
    run_malloc_index_store_load => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); arr[0] = 11; print(arr[0]); free(arr); }", "11\n");
    run_malloc_index_one => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); arr[1] = 22; print(arr[1]); free(arr); }", "22\n");
    run_array_set_get_zero => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); array_set(arr, 0, 33); print(array_get(arr, 0)); free(arr); }", "33\n");
    run_array_set_get_one => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); array_set(arr, 1, 44); print(array_get(arr, 1)); free(arr); }", "44\n");
    run_push_pop_one => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 2); i32: len = 0; push(arr, &len, 55); print(pop(arr, &len)); free(arr); }", "55\n");
    run_push_pop_lifo => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 4); i32: len = 0; push(arr, &len, 66); push(arr, &len, 77); print(pop(arr, &len)); print(pop(arr, &len)); free(arr); }", "77\n66\n");
    run_push_then_array_get => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 4); i32: len = 0; push(arr, &len, 88); print(array_get(arr, 0)); free(arr); }", "88\n");
    run_index_variable => ("fn void run(){ i32: *arr = malloc(sizeof(i32) * 4); i32: i = 1; arr[i] = 99; print(arr[i]); free(arr); }", "99\n");
    run_many_prints => ("fn void run(){ print(sizeof(i32)); i32: *arr = malloc(sizeof(i32) * 10); i32: len = 0; arr[0] = 11; print(arr[0]); array_set(arr, 1, 22); print(array_get(arr, 1)); push(arr, &len, 33); push(arr, &len, 44); print(pop(arr, &len)); print(pop(arr, &len)); free(arr); }", "4\n11\n22\n44\n33\n");
    run_pointer_return_store_load => ("fn *i32 make(){ i32: *p = malloc(sizeof(i32)); return p; } fn void run(){ i32: *p = make(); *p = 123; print(*p); free(p); }", "123\n");
}
