//! Command line entry: swe-diagrams render <file|-> [-o out.svg] [--icons dir] [--offline]

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use swe_diagrams::{render, IconOptions};

const USAGE: &str = "usage: swe-diagrams render <file.sd|-> [-o out.svg] [--icons dir] [--offline]\n";
const VERSION: &str = concat!("swe-diagrams ", env!("CARGO_PKG_VERSION"), "\n");

// A closed pipe must not cause a panic, so the two functions report a
// failure to the caller.
fn to_stdout(text: &str) -> bool {
    let mut out = io::stdout().lock();
    out.write_all(text.as_bytes()).and_then(|()| out.flush()).is_ok()
}

fn to_stderr(text: &str) {
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

fn read_input(input: &OsString) -> io::Result<Vec<u8>> {
    if input == "-" {
        let mut bytes = Vec::new();
        io::stdin().lock().read_to_end(&mut bytes)?;
        Ok(bytes)
    } else {
        std::fs::read(input)
    }
}

fn run(argv: &[OsString]) -> u8 {
    if argv.len() == 1 && argv[0] == "--version" {
        return if to_stdout(VERSION) { 0 } else { 1 };
    }
    if argv.len() < 2 || argv[0] != "render" {
        to_stderr(USAGE);
        return 1;
    }
    let input = &argv[1];
    let shown = input.to_string_lossy();
    // SWE_DIAGRAMS_OFFLINE=1 has the same effect as --offline: no logo fetch.
    let mut opts =
        IconOptions { dir: None, offline: std::env::var_os("SWE_DIAGRAMS_OFFLINE").is_some_and(|v| v == "1") };
    let mut output: Option<&OsString> = None;
    let mut k = 2;
    while k < argv.len() {
        if argv[k] == "-o" && k + 1 < argv.len() {
            // An empty output name means standard output.
            output = Some(&argv[k + 1]).filter(|o| !o.is_empty());
            k += 2;
        } else if argv[k] == "--icons" && k + 1 < argv.len() {
            opts.dir = Some(std::path::PathBuf::from(&argv[k + 1]));
            k += 2;
        } else if argv[k] == "--offline" {
            opts.offline = true;
            k += 1;
        } else {
            to_stderr(USAGE);
            return 1;
        }
    }
    let Ok(bytes) = read_input(input) else {
        to_stderr(&format!("error: cannot read {shown}\n"));
        return 1;
    };
    // A byte sequence that is not UTF-8 becomes U+FFFD.
    let src = String::from_utf8_lossy(&bytes);
    match render(&src, &opts) {
        Ok(r) => match output {
            None => {
                let ok = to_stdout(&r.svg);
                to_stderr(&format!("{}\n", r.summary));
                if ok {
                    0
                } else {
                    1
                }
            }
            Some(output) => {
                if std::fs::write(output, &r.svg).is_err() {
                    to_stderr(&format!("error: cannot write {}\n", output.to_string_lossy()));
                    return 1;
                }
                if to_stdout(&format!("{} {}\n", r.summary, output.to_string_lossy())) {
                    0
                } else {
                    1
                }
            }
        },
        Err(err) => {
            let mut text = String::new();
            for m in &err.errors {
                text.push_str(&format!("{shown}:{m}\n"));
            }
            to_stderr(&text);
            2
        }
    }
}

fn main() -> ExitCode {
    let argv: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(&argv))
}
