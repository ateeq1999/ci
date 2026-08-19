use clap::{ArgAction, CommandFactory, Parser, Subcommand};

use crate::commands::{add, db, init, update};

#[derive(Parser)]
#[command(name = "ci", author, version, about, long_about = None, disable_version_flag = true)]
pub struct Cli {
    /// Provide this command's arguments as a JSON payload instead of flags.
    /// Accepts a literal JSON string, or `@path/to/file.json` to read one.
    #[arg(short = 'j', long, global = true)]
    pub json: Option<String>,

    /// Print version
    #[arg(short = 'v', long = "version", action = ArgAction::Version)]
    version: (),

    /// Print every command, subcommand, and flag in one tree, then exit.
    /// Checked before normal parsing, so it works with no subcommand given
    /// (unlike `--help`, which needs one).
    #[arg(long = "help-all", global = true)]
    #[allow(dead_code)]
    help_all: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new NestJS project
    Init(init::Args),
    /// Update this CLI to the latest release
    Update(update::Args),
    /// Run database migration commands (see db.md)
    Db(db::Args),
    /// Wire a NestJS technique into an existing project (see plan.md)
    Add(add::Args),
}

/// Checked directly against `std::env::args()` by `main`, before
/// `Cli::parse()` — parsing would otherwise fail on `ci --help-all` alone
/// (`command` is a required subcommand) or ignore the flag if it's not
/// itself what triggers the exit.
pub fn wants_help_all() -> bool {
    std::env::args().any(|a| a == "--help-all")
}

/// Prints every command, subcommand, and flag as one indented tree.
pub fn print_full_help() {
    print_command_tree(&Cli::command(), 0);
}

fn print_command_tree(cmd: &clap::Command, depth: usize) {
    let indent = "  ".repeat(depth);
    let about = cmd.get_about().map(ToString::to_string).unwrap_or_default();
    println!("{indent}{} — {about}", cmd.get_name());

    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if id == "help" || id == "version" || id == "help_all" {
            continue;
        }
        let flag = if arg.is_positional() {
            format!("<{}>", arg.get_id())
        } else {
            let short = arg.get_short().map(|c| format!("-{c}"));
            let long = arg.get_long().map(|l| format!("--{l}"));
            [short, long].into_iter().flatten().collect::<Vec<_>>().join(", ")
        };
        if flag.is_empty() {
            continue;
        }
        let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
        println!("{indent}    {flag:<20} {help}");
    }

    for sub in cmd.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        println!();
        print_command_tree(sub, depth + 1);
    }
}
