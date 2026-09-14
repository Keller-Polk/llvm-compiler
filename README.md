# Compiler

A small C-like programming language compiler written in Rust. It lexes, parses, builds an AST, generates LLVM IR ,and emits an object file, and links it into a native executable.

The project is currently experimental, but it already supports all the basic components of a simple language like functions, variables, pointers, memory allocation, printing, loops, conditionals, and simple pointer-backed arrays.

it generates LLVM IR with [`inkwell`](https://github.com/TheDan64/inkwell).

## Features

- Integer, unsigned integer, float, bool, char, void, pointer, and array types
- Global and local variables
- Functions with parameters and return values
- Pointer syntax:
  - address-of: `&x`
  - dereference: `*ptr`
  - pointer assignment: `*ptr = 123;`
- Arithmetic and comparisons
- `if` / `else`
- `while`
- `break` / `continue`
- `sizeof(type)`
- Built-in `print(...)`
- Built-in libc-style memory functions:
  - `malloc(...)`
  - `free(...)`
- Pointer-backed indexing:
  - `arr[0] = 11;`
  - `print(arr[0]);`
- Simple array helper built-ins:
  - `array_get(arr, index)`
  - `array_set(arr, index, value)`
  - `push(arr, &len, value)`
  - `pop(arr, &len)`

## Example

```c
fn *i32 make_array(){
    i32: *arr = malloc(sizeof(i32) * 10);
    return arr;
}

fn void run(){
    print(sizeof(i32));

    i32: *arr = make_array();
    i32: len = 0;

    arr[0] = 11;
    print(arr[0]);

    array_set(arr, 1, 22);
    print(array_get(arr, 1));

    push(arr, &len, 33);
    push(arr, &len, 44);

    print(pop(arr, &len));
    print(pop(arr, &len));

    free(arr);
}
```

Expected output:

```text
4
11
22
44
33
```

## Download

Prebuilt downloads are included in this repository/release:

### Linux

Download the Linux binary named:

```text
compiler
```


Run it on a source file:

```sh
./compiler test.lang
```

### Windows

Download and extract:

```text
windows.zip
```

Then run the compiler executable from the extracted folder:

```powershell
.\compiler.exe test.lang
```

## Usage

Compile a language file:

```sh
./compiler path/to/file.lang
```

or:

```sh
./compiler --filepath path/to/file.lang
```


The output executable uses the same base name as the input file:

| Input file | Linux output | Windows output |
|---|---|---|
| `test.lang` | `test` | `test.exe` |
| `examples/demo.lang` | `examples/demo` | `examples/demo.exe` |

Run the compiled program:

```sh
./test
```

On Windows:

```powershell
.\test.exe
```

## Building from source

Requirements:

- Rust/Cargo
- LLVM 18 development libraries
- A C compiler/linker available as `cc`

Clone the repository:

```sh
git clone <https://github.com/Keller-Polk/llvm-compiler>
cd compiler
```

Build:

```sh
cargo build --release
```

Run from source:

```sh
cargo run -- test.lang
```

or:

```sh
cargo run -- --filepath test.lang
```


## Testing

Run the test suite:

```sh
cargo test
```


## Current limitations

This compiler is still early and intentionally simple. Some rough edges remain:

- Error messages are improving but are not fully polished yet.
- `push`/`pop` do not do capacity checks or automatic reallocation.
- Memory management is manual: if you `malloc`, you should eventually `free`.

## Project layout

```text
src/lexer.rs    tokenizes source code
src/parser.rs   parses tokens into AST nodes
src/builder.rs  generates LLVM IR, object files, and executables
src/main.rs     command-line entry point
tests/          parser/codegen/runtime tests
```
