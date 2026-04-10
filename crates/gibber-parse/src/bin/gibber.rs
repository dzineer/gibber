use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "validate" => cmd_validate(&args[2..]),
        "rebuild-index" => cmd_rebuild_index(&args[2..]),
        "parse" => cmd_parse(&args[2..]),
        "help" | "--help" | "-h" => print_usage(),
        other => {
            eprintln!("(§result §tool:gibber §outcome:§failed §error:\"unknown command: {}\")", other);
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("gibber — Gibber file tooling");
    println!();
    println!("COMMANDS:");
    println!("  validate <file|dir>     Validate .gibber files (parse, round-trip, structure)");
    println!("  rebuild-index <dir>     Rebuild tasks_index.gibber from T*.gibber files");
    println!("  parse <file>            Parse a .gibber file and print the AST as JSON");
    println!("  help                    Show this help");
}

fn cmd_validate(args: &[String]) {
    if args.is_empty() {
        eprintln!("(§result §tool:gibber §cmd:validate §outcome:§failed §error:\"no path given\")");
        process::exit(1);
    }

    let path = PathBuf::from(&args[0]);
    let mut total_files = 0;
    let mut total_errors = 0;
    let mut total_warnings = 0;

    if path.is_dir() {
        // Validate all .gibber files in directory (recursive)
        validate_dir(&path, &mut total_files, &mut total_errors, &mut total_warnings);
    } else {
        // Validate single file
        total_files = 1;
        let issues = gibber_parse::validate_file(&path);
        print_issues(&path, &issues);
        for issue in &issues {
            match issue.severity {
                gibber_parse::validate::Severity::Error => total_errors += 1,
                gibber_parse::validate::Severity::Warning => total_warnings += 1,
            }
        }
    }

    let outcome = if total_errors > 0 { "§failed" } else { "§passed" };
    println!(
        "(§result §tool:gibber §cmd:validate §outcome:{} §files:{} §errors:{} §warnings:{})",
        outcome, total_files, total_errors, total_warnings
    );

    if total_errors > 0 {
        process::exit(1);
    }
}

fn validate_dir(dir: &PathBuf, files: &mut usize, errors: &mut usize, warnings: &mut usize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        if path.is_dir() {
            validate_dir(&path, files, errors, warnings);
        } else if path.extension().map_or(false, |e| e == "gibber") {
            *files += 1;
            let issues = gibber_parse::validate_file(&path);
            if !issues.is_empty() {
                print_issues(&path, &issues);
            }
            for issue in &issues {
                match issue.severity {
                    gibber_parse::validate::Severity::Error => *errors += 1,
                    gibber_parse::validate::Severity::Warning => *warnings += 1,
                }
            }
        }
    }
}

fn print_issues(path: &PathBuf, issues: &[gibber_parse::validate::ValidationIssue]) {
    for issue in issues {
        eprintln!("  {} — {}", path.display(), issue);
    }
}

fn cmd_rebuild_index(args: &[String]) {
    let dir = if args.is_empty() {
        PathBuf::from("tasks")
    } else {
        PathBuf::from(&args[0])
    };

    match gibber_parse::indexer::rebuild_index(&dir) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("(§result §tool:gibber §cmd:rebuild-index §outcome:§failed §error:\"{}\")", e);
            process::exit(1);
        }
    }
}

fn cmd_parse(args: &[String]) {
    if args.is_empty() {
        eprintln!("(§result §tool:gibber §cmd:parse §outcome:§failed §error:\"no file given\")");
        process::exit(1);
    }

    let path = PathBuf::from(&args[0]);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("(§result §tool:gibber §cmd:parse §outcome:§failed §error:\"{}\")", e);
            process::exit(1);
        }
    };

    match gibber_parse::parse(&content) {
        Ok(file) => {
            let json = serde_json::to_string_pretty(&file).unwrap();
            println!("{}", json);
        }
        Err(e) => {
            eprintln!("(§result §tool:gibber §cmd:parse §outcome:§failed §error:\"{}\")", e);
            process::exit(1);
        }
    }
}
