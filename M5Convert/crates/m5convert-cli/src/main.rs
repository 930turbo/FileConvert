use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use m5convert_core::{ConvertOptions, FORMATS, Planner, Toolset, convert_file, format_by_extension, format_by_id};

#[derive(Parser)]
#[command(name = "m5convert", version, about = "Local file conversion for Windows")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert one or more files beside the originals.
    Convert {
        #[arg(long)]
        to: String,
        #[arg(long, default_value_t = 92)]
        quality: u8,
        #[arg(long)]
        overwrite: bool,
        /// Show a native error dialog instead of silently returning an error.
        #[arg(long)]
        notify: bool,
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Print all recognized formats.
    Formats,
    /// Print reachable targets for a file.
    Targets {
        file: PathBuf,
        #[arg(long)]
        menu: bool,
    },
    /// Show which optional engines are currently available.
    Engines,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((notify, err)) => {
            if notify { native_error(&err.to_string()); }
            else { eprintln!("m5convert: {err:#}"); }
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), (bool, anyhow::Error)> {
    let cli = Cli::parse();
    let notify = matches!(&cli.command, Command::Convert { notify: true, .. });
    execute(cli).map_err(|e| (notify, e))
}

fn execute(cli: Cli) -> anyhow::Result<()> {
    let tools = Toolset::discover();
    let planner = Planner::new(tools);
    match cli.command {
        Command::Formats => {
            for f in FORMATS {
                println!("{:<6} {:<13?} {}", f.id, f.category, f.name);
            }
        }
        Command::Targets { file, menu } => {
            let from = format_by_extension(&file)
                .with_context(|| format!("unsupported file: {}", file.display()))?;
            let list = if menu { planner.menu_targets(from) } else { planner.targets(from) };
            for f in list { println!("{}", f.id); }
        }
        Command::Engines => {
            for (name, available, detail) in planner.tools().summary() {
                let state = if available { "ready" } else { "missing" };
                match detail {
                    Some(d) => println!("{name:<14} {state:<7} {d}"),
                    None => println!("{name:<14} {state}"),
                }
            }
        }
        Command::Convert { to, quality, overwrite, notify: _, files } => {
            if format_by_id(&to).is_none() { bail!("unknown target format: {to}"); }
            if !(1..=100).contains(&quality) { bail!("quality must be 1..100"); }
            let options = ConvertOptions { jpeg_quality: quality, overwrite };
            let mut failed = Vec::new();
            for file in files {
                match convert_file(&planner, &file, &to, &options) {
                    Ok(outputs) => {
                        for out in outputs { println!("{}", out.display()); }
                    }
                    Err(e) => failed.push(format!("{}: {e}", file.display())),
                }
            }
            if !failed.is_empty() {
                bail!("{}", failed.join("\n"));
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn native_error(message: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::HSTRING;
    let body = HSTRING::from(message);
    let title = HSTRING::from("M5Convert");
    unsafe { let _ = MessageBoxW(None, &body, &title, MB_OK | MB_ICONERROR); }
}

#[cfg(not(windows))]
fn native_error(message: &str) { eprintln!("m5convert: {message}"); }
