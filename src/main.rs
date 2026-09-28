mod render;

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
md2html - convert Markdown to a self-contained, print-ready HTML document

Usage:
  md2html [OPTIONS] [FILE]

Arguments:
  FILE                  Markdown input; omitted or '-' reads stdin

Options:
  -o, --output <FILE>   Write to FILE instead of stdout
  -t, --title <TEXT>    Document title (default: first level-1 heading, else the
                        input file name, else 'Document')
  -f, --fragment        Emit the rendered body only: no wrapper, no styles
      --css <FILE>      Use FILE as the stylesheet instead of the built-in one
      --no-css          Standalone document with no stylesheet
      --dump-css        Print the built-in stylesheet and exit
  -h, --help            Show this help
  -V, --version         Show version

Output is built for paper: print or save-as-PDF from the browser and you get
sane page margins, headings that stay with their text, code blocks that wrap
instead of clipping, and link destinations spelled out inline.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("md2html: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Where the stylesheet of a standalone document comes from.
enum Style {
    Builtin,
    File(PathBuf),
    None,
}

struct Args {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    title: Option<String>,
    fragment: bool,
    style: Style,
}

/// What the command line asked for; the informational modes short-circuit `run`.
enum Request {
    Help,
    Version,
    DumpCss,
    Convert(Args),
}

fn run() -> Result<(), String> {
    match parse_args(std::env::args().skip(1))? {
        Request::Help => write_out(None, USAGE.as_bytes()),
        Request::Version => write_out(
            None,
            format!("md2html {}\n", env!("CARGO_PKG_VERSION")).as_bytes(),
        ),
        Request::DumpCss => write_out(None, render::DEFAULT_CSS.as_bytes()),
        Request::Convert(args) => convert(args),
    }
}

fn convert(args: Args) -> Result<(), String> {
    let markdown = read_input(args.input.as_deref())?;

    let html = if args.fragment {
        render::to_fragment(&markdown)
    } else {
        let title = args
            .title
            .or_else(|| render::first_h1(&markdown))
            .or_else(|| file_stem(args.input.as_deref()))
            .unwrap_or_else(|| "Document".to_string());

        let css = match &args.style {
            Style::Builtin => Some(render::DEFAULT_CSS.to_string()),
            Style::File(path) => Some(read_file(path)?),
            Style::None => None,
        };

        render::to_document(&markdown, &title, css.as_deref())
    };

    write_out(args.output.as_deref(), html.as_bytes())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Request, String> {
    let mut parsed = Args {
        input: None,
        output: None,
        title: None,
        fragment: false,
        style: Style::Builtin,
    };
    let mut no_css = false;
    let mut css_file: Option<PathBuf> = None;
    let mut args = args;
    let mut flags_done = false;

    while let Some(arg) = args.next() {
        // A lone "-" means stdin, so it is a positional, not a flag.
        if flags_done || arg == "-" || !arg.starts_with('-') {
            if parsed.input.is_some() {
                return Err(format!("unexpected extra argument '{arg}'"));
            }
            if arg != "-" {
                parsed.input = Some(PathBuf::from(arg));
            }
            continue;
        }

        let mut value = |flag: &str| -> Result<String, String> {
            args.next()
                .ok_or_else(|| format!("'{flag}' needs a value (see --help)"))
        };

        match arg.as_str() {
            "-h" | "--help" => return Ok(Request::Help),
            "-V" | "--version" => return Ok(Request::Version),
            "--dump-css" => return Ok(Request::DumpCss),
            "-f" | "--fragment" => parsed.fragment = true,
            "--no-css" => no_css = true,
            "-o" | "--output" => parsed.output = Some(PathBuf::from(value(&arg)?)),
            "-t" | "--title" => parsed.title = Some(value(&arg)?),
            "--css" => css_file = Some(PathBuf::from(value(&arg)?)),
            "--" => flags_done = true,
            _ => return Err(format!("unknown option '{arg}' (see --help)")),
        }
    }

    parsed.style = match (no_css, css_file) {
        (true, Some(_)) => return Err("'--css' and '--no-css' conflict".to_string()),
        (true, None) => Style::None,
        (false, Some(path)) => Style::File(path),
        (false, None) => Style::Builtin,
    };

    Ok(Request::Convert(parsed))
}

fn read_input(path: Option<&Path>) -> Result<String, String> {
    match path {
        Some(path) => read_file(path),
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("reading stdin: {e}"))?;
            Ok(buf)
        }
    }
}

fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

fn write_out(path: Option<&Path>, bytes: &[u8]) -> Result<(), String> {
    match path {
        Some(path) => {
            fs::write(path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))
        }
        None => {
            let mut stdout = io::stdout().lock();
            match stdout.write_all(bytes).and_then(|()| stdout.flush()) {
                // `md2html big.md | head` closing the pipe early is not an error.
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
                result => result.map_err(|e| format!("writing stdout: {e}")),
            }
        }
    }
}

fn file_stem(path: Option<&Path>) -> Option<String> {
    let stem = path?.file_stem()?.to_string_lossy().into_owned();
    (!stem.is_empty()).then_some(stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Request, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    fn convert_args(args: &[&str]) -> Args {
        match parse(args) {
            Ok(Request::Convert(a)) => a,
            _ => panic!("expected a conversion request from {args:?}"),
        }
    }

    #[test]
    fn parses_flags_and_positional() {
        let a = convert_args(&["-o", "out.html", "-t", "Title", "in.md"]);
        assert_eq!(a.input.unwrap(), Path::new("in.md"));
        assert_eq!(a.output.unwrap(), Path::new("out.html"));
        assert_eq!(a.title.unwrap(), "Title");
        assert!(!a.fragment);
        assert!(matches!(a.style, Style::Builtin));
    }

    #[test]
    fn dash_and_absence_both_mean_stdin() {
        assert!(convert_args(&["-"]).input.is_none());
        assert!(convert_args(&[]).input.is_none());
    }

    #[test]
    fn double_dash_ends_flag_parsing() {
        let a = convert_args(&["--", "-weird-name.md"]);
        assert_eq!(a.input.unwrap(), Path::new("-weird-name.md"));
    }

    #[test]
    fn informational_modes_win() {
        assert!(matches!(parse(&["-h"]), Ok(Request::Help)));
        assert!(matches!(parse(&["in.md", "-V"]), Ok(Request::Version)));
        assert!(matches!(parse(&["--dump-css"]), Ok(Request::DumpCss)));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse(&["--nope"]).is_err());
        assert!(parse(&["-o"]).is_err());
        assert!(parse(&["a.md", "b.md"]).is_err());
        assert!(parse(&["--css", "x.css", "--no-css"]).is_err());
    }

    #[test]
    fn style_selection() {
        assert!(matches!(convert_args(&["--no-css"]).style, Style::None));
        assert!(matches!(
            convert_args(&["--css", "x.css"]).style,
            Style::File(_)
        ));
    }

    #[test]
    fn stem_used_for_title_fallback() {
        assert_eq!(
            file_stem(Some(Path::new("docs/notes.md"))).as_deref(),
            Some("notes")
        );
        assert_eq!(file_stem(None), None);
    }
}
