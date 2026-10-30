// SPDX-License-Identifier: MIT
//! `craft`: the claude-craft-kit toolbox. One binary, one subcommand per former script (same names, flags, output and exit codes).

mod args;
mod classify_structure;
mod map_reuse;
mod network;
mod regression_diff;
mod standards;
mod sync_core;
mod testenv;
mod text;
mod verify;

const USAGE: &str = "craft: the claude-craft-kit toolbox

Usage: craft <tool> [arguments]

Tools:
  testenv             test credentials: the project's .env.test, machine sources, network list, exec with redaction
  verify              detects and runs typecheck, lint, build and test; prints PASS, FAIL or UNVERIFIED
  standards           `standards check`: the deterministic rules of the skills (rules.json) on the changed files
  regression_diff     compares two JSON snapshots (baseline x new) after normalising volatile data
  map_reuse           prints a DRAFT of the 'Reuse map' section of a project's CLAUDE.md
  classify_structure  classifies a project's structure as STANDARD or LEGACY and writes the Conformance section
  sync_core           injects shared/rules/core-dev.md into the backend-dev and frontend-dev skills

Run `craft <tool> --help` for the details of one tool.
";

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&argv);
    text::flush_all();
    std::process::exit(code);
}

fn run(argv: &[String]) -> i32 {
    let Some((tool, rest)) = argv.split_first() else {
        eprint!("{USAGE}");
        return 2;
    };
    match tool.as_str() {
        "testenv" => testenv::main(rest),
        "verify" => verify::main(rest),
        "standards" => standards::main(rest),
        "regression_diff" | "regression-diff" => regression_diff::main(rest),
        "map_reuse" | "map-reuse" => map_reuse::main(rest),
        "classify_structure" | "classify-structure" => classify_structure::main(rest),
        "sync_core" | "sync-core" => sync_core::main(rest),
        "--version" | "-V" | "version" => {
            println!("craft {}", env!("CARGO_PKG_VERSION"));
            0
        }
        "--help" | "-h" | "help" => {
            print!("{USAGE}");
            0
        }
        other => {
            eprintln!("craft: unknown tool `{other}`\n");
            eprint!("{USAGE}");
            2
        }
    }
}
