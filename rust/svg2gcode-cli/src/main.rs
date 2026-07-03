//! Port of cli/rootCmd.go + main.go.
//!
//! Scope trim vs. Go (documented, not silent): Go's CLI has a `-v`
//! verbosity flag controlling a leveled (SILLY/TRACE/DEBUG/WARN/ERROR)
//! logging system threaded through parsing and carving. None of that
//! logging was ported to the svg2gcode library (no log statements exist
//! anywhere in it) -- it's Go-specific troubleshooting output that no test
//! or behavior depends on, and porting a full leveled-logging system
//! wasn't part of the sentinel-depth work this port exists for. A `-v`
//! flag with nothing to control would just be dead CLI surface, so it's
//! dropped rather than faked.
//!
//! This binary uses the default (non-probe) DepthResolver: it errors on
//! any carve-depth sentinel. xcarve-controller supplies its own resolver
//! when consuming svg2gcode as a library.

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use svg2gcode::document::parse_svg_document;
use svg2gcode::gcode_desc::{CutContext, DepthResolver, ResolveError};
use svg2gcode::gcode_writer::GCodeWriter;
use svg2gcode::svgx_document::load_document;

/// svg2gcode helps generate gcode from svg files.
#[derive(Parser)]
#[command(name = "svg2gcode", version, about = "Converts svg files to gcode.")]
struct Args {
    /// Input SVG file(s). If none are given, reads from stdin.
    inputs: Vec<PathBuf>,

    /// Output filename. If not provided, writes to stdout.
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,
}

/// The standalone CLI has no probe or height-map support, so any
/// carve-depth sentinel (e.g. "full") is unresolvable here -- this always
/// errors, aborting gcode generation for that document. svg2gcode never
/// substitutes a default depth on its own.
struct NoSentinelResolver;

impl DepthResolver for NoSentinelResolver {
    fn resolve(&self, sentinel: &str, ctx: &CutContext) -> Result<f64, ResolveError> {
        Err(ResolveError(format!(
            "cannot resolve carve-depth sentinel '{sentinel}' at ({:.3}, {:.3})mm: the \
             standalone CLI has no probe/height-map support. Use a fixed numeric carve-depth \
             (e.g. \"10mm\") in the SVG, or consume svg2gcode as a library with a DepthResolver \
             that knows how to resolve '{sentinel}'.",
            ctx.x, ctx.y
        )))
    }
}

fn read_to_string(mut input: impl Read) -> std::io::Result<String> {
    let mut text = String::new();
    input.read_to_string(&mut text)?;
    Ok(text)
}

fn main() -> ExitCode {
    let args = Args::parse();

    let sources: Vec<(String, String)> = if args.inputs.is_empty() {
        match read_to_string(std::io::stdin()) {
            Ok(text) => vec![("stdin".to_string(), text)],
            Err(err) => {
                eprintln!("error: failed to read stdin: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let mut sources = Vec::with_capacity(args.inputs.len());
        for path in &args.inputs {
            let name = path.display().to_string();
            let file = match File::open(path) {
                Ok(f) => f,
                Err(err) => {
                    eprintln!("error: failed to open {name}: {err}");
                    return ExitCode::FAILURE;
                }
            };
            match read_to_string(file) {
                Ok(text) => sources.push((name, text)),
                Err(err) => {
                    eprintln!("error: failed to read {name}: {err}");
                    return ExitCode::FAILURE;
                }
            }
        }
        sources
    };

    let output: Box<dyn Write> = match &args.output {
        Some(path) => match File::create(path) {
            Ok(f) => Box::new(f),
            Err(err) => {
                eprintln!("error: failed to open {} for writing: {err}", path.display());
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(std::io::stdout()),
    };

    let mut writer = GCodeWriter::new(output);
    let resolver = NoSentinelResolver;

    for (name, text) in sources {
        let doc = match parse_svg_document(&text) {
            Ok(doc) => doc,
            Err(err) => {
                eprintln!("error: failed to parse {name}: {err}");
                return ExitCode::FAILURE;
            }
        };
        let svgx_doc = match load_document(name.clone(), &doc) {
            Ok(d) => d,
            Err(err) => {
                eprintln!("error: failed to load {name}: {err}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(err) = svgx_doc.carve(&mut writer, &resolver) {
            eprintln!("error: failed to carve {name}: {err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
