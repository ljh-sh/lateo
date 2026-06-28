//! lateo CLI — imperceptible data embedding (steganography + watermarking).
//!
//! Thin dispatch over [`lateo::Op`]. Parsing is hand-rolled (no clap) while
//! the verbs are stubs; pull in `clap` once a verb grows real flags.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let prog = args.first().map(String::as_str).unwrap_or("lateo");

    match args.get(1).map(String::as_str) {
        None => {
            print_usage(prog);
            ExitCode::from(2)
        }
        Some("-h") | Some("--help") => {
            print_usage(prog);
            ExitCode::SUCCESS
        }
        Some("-V") | Some("--version") => {
            println!("lateo {}", lateo::VERSION);
            ExitCode::SUCCESS
        }
        Some(verb) => match lateo::Op::parse(verb) {
            Some(op) => {
                // TODO: dispatch to the steganography / watermarking engines.
                eprintln!("lateo: '{verb}' is wired but not implemented yet (scaffold).");
                eprintln!("       planned op: {op:?}");
                ExitCode::FAILURE
            }
            None => {
                eprintln!("lateo: unknown subcommand '{verb}'");
                print_usage(prog);
                ExitCode::from(2)
            }
        },
    }
}

fn print_usage(prog: &str) {
    eprintln!(
        "lateo — imperceptible data embedding (steganography + watermarking)

usage: {prog} <subcommand> [options]

subcommands:
  hide      embed a covert payload   (steganography — fragile, high-capacity)
  extract   recover a covert payload
  mark      embed a robust imprint    (watermark — robust, low-capacity)
  verify    check a robust imprint
  probe     reverse self-check (can I be detected / stripped?)

flags:
  -V, --version   print version
  -h, --help      print this help"
    );
}
