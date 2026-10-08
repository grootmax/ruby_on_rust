use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ruby_guard_linter <path-to-rust-file>...");
        process::exit(1);
    }

    let mut total_errors = 0;
    for file_path in &args[1..] {
        match ruby_guard_linter::lint_file(file_path) {
            Ok(diagnostics) => {
                for diag in diagnostics {
                    eprintln!(
                        "error[Rule 6]: {}\n  --> {}:{}",
                        diag.message, file_path, diag.line
                    );
                    total_errors += 1;
                }
            }
            Err(err) => {
                eprintln!("Failed to parse/lint {}: {}", file_path, err);
                process::exit(1);
            }
        }
    }

    if total_errors > 0 {
        eprintln!("\nFound {} Rule 6 exception safety violation(s).", total_errors);
        process::exit(1);
    } else {
        println!("No Rule 6 exception safety violations found.");
    }
}
