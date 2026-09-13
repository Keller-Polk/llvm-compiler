use compiler::builder::*;
use compiler::lexer::*;
use compiler::parser::*;
use std::env;
use std::path::Path;
use std::process;

fn filepath_from_args() -> String {
    let mut args = env::args().skip(1);
    let mut filepath = None;

    while let Some(arg) = args.next() {
        if arg == "--filepath" {
            filepath = Some(args.next().unwrap_or_else(|| {
                eprintln!("Compiler Error: --filepath requires a path argument.");
                process::exit(1);
            }));
            continue;
        }

        if let Some(path) = arg.strip_prefix("--filepath=") {
            if path.is_empty() {
                eprintln!("Compiler Error: --filepath requires a non-empty path.");
                process::exit(1);
            }
            filepath = Some(path.to_string());
            continue;
        }

        if arg == "--help" || arg == "-h" {
            println!("Usage: compiler [filepath]");
            println!("       compiler --filepath <path>");
            println!("Default filepath: src/test.lang");
            process::exit(0);
        }

        if arg.starts_with('-') {
            eprintln!("Compiler Error: unknown argument '{}'.", arg);
            eprintln!("Usage: compiler [filepath]");
            eprintln!("       compiler --filepath <path>");
            process::exit(1);
        }

        if filepath.is_some() {
            eprintln!("Compiler Error: only one input filepath can be provided.");
            process::exit(1);
        }

        filepath = Some(arg);
    }

    filepath.unwrap_or_else(|| "src/test.lang".to_string())
}

fn main() {
    let filepath = filepath_from_args();
    let y = Token::load_file(&filepath);
    let mut t = TokenParse::convert_from_lex(y);
    let program = t.parse();

    let exe_extension = if cfg!(target_os = "windows") {
        "exe"
    } else {
        ""
    };

    let input_path = Path::new(&filepath);
    let output_path = if exe_extension.is_empty() {
        // Linux/macOS: strip extension completely (e.g., "src/test")
        input_path.with_extension("")
    } else {
        // Windows: change extension to .exe (e.g., "src/test.exe")
        input_path.with_extension(exe_extension)
    };

    let output_name = output_path.to_string_lossy().to_string();

    program.build(&output_name);
}
