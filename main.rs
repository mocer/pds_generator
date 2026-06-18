use clap::{Arg, Command};
use std::fmt;
use std::io::{self, BufWriter, Write};

// ── Error types ───────────────────────────────────────────────────────────────

#[derive(Debug)]
enum PdsError {
    InvalidLength(usize),
    InvalidPrefix(u8),
    InvalidChar(u8),
    Overflow,
}

impl fmt::Display for PdsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdsError::InvalidLength(n) => write!(f, "PDS must be 5 bytes, got {n}"),
            PdsError::InvalidPrefix(c) => {
                write!(f, "PDS must start with 'W', got '{}'", *c as char)
            }
            PdsError::InvalidChar(c) => write!(f, "Invalid character '{}' in PDS", *c as char),
            PdsError::Overflow => write!(f, "PDS overflow: exceeded maximum WZZZZ"),
        }
    }
}

impl std::error::Error for PdsError {}

// ── Output format ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum OutputFormat {
    Csv,
    Sql,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Csv => write!(f, "CSV"),
            OutputFormat::Sql => write!(f, "SQL"),
        }
    }
}

// ── PDS logic ─────────────────────────────────────────────────────────────────

/// Validates a PDS code returning a fixed-size byte buffer.
///
/// A valid PDS is exactly 5 ASCII bytes, starting with `W`,
/// followed by 4 characters in `[0-9A-Z]`.
fn parse_pds(input: &str) -> Result<[u8; 5], PdsError> {
    let bytes = input.as_bytes();

    if bytes.len() != 5 {
        return Err(PdsError::InvalidLength(bytes.len()));
    }
    if bytes[0] != b'W' {
        return Err(PdsError::InvalidPrefix(bytes[0]));
    }
    for &b in &bytes[1..] {
        if !matches!(b, b'0'..=b'9' | b'A'..=b'Z') {
            return Err(PdsError::InvalidChar(b));
        }
    }

    Ok(bytes.try_into().unwrap()) // safe: length already checked
}

/// Increments a PDS code in-place using base-36 arithmetic (`[0-9A-Z]`).
///
/// The leading `W` is fixed. Carry propagates right-to-left over positions 1–4.
/// Returns [`PdsError::Overflow`] if the value exceeds `WZZZZ`.
fn increment_pds(pds: &mut [u8; 5]) -> Result<(), PdsError> {
    for i in (1..5).rev() {
        match pds[i] {
            b'0'..=b'8' => {
                pds[i] += 1;
                return Ok(());
            }
            b'9' => {
                pds[i] = b'A';
                return Ok(());
            }
            b'A'..=b'Y' => {
                pds[i] += 1;
                return Ok(());
            }
            b'Z' => {
                pds[i] = b'0';
            }
            c => return Err(PdsError::InvalidChar(c)),
        }
    }
    Err(PdsError::Overflow)
}

// ── CLI ───────────────────────────────────────────────────────────────────────

fn parse_format(s: &str) -> Result<OutputFormat, String> {
    match s.to_uppercase().as_str() {
        "CSV" => Ok(OutputFormat::Csv),
        "SQL" => Ok(OutputFormat::Sql),
        other => Err(format!("Unknown format '{other}'. Use CSV or SQL.")),
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("pds_generator")
        .version("1.0")
        .about("Generates sequential PDS codes starting after a given PDS")
        .arg(
            Arg::new("start_pds")
                .help("Starting PDS code (excluded from output), e.g. W18A1")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("quantity")
                .help("Number of PDS codes to generate")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::new("format")
                .help("Output format: CSV (default) or SQL")
                .required(false)
                .index(3),
        )
        .get_matches();

    // -- parse start PDS
    let start_raw = matches.get_one::<String>("start_pds").unwrap(); // safe: required
    let mut pds = parse_pds(start_raw)?;

    // -- parse quantity
    let quantity: usize = matches
        .get_one::<String>("quantity")
        .unwrap() // safe: required
        .parse()
        .map_err(|_| "QUANTITY must be a positive integer")?;

    // -- parse format
    let format = match matches.get_one::<String>("format") {
        Some(s) => parse_format(s)?,
        None => OutputFormat::Csv,
    };

    // -- output
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    match format {
        OutputFormat::Csv => {
            writeln!(out, "current_pds_code,flag_utilizzo,data_utilizzo")?;
            for _ in 0..quantity {
                increment_pds(&mut pds)?;
                // SAFETY: pds contains only ASCII bytes validated on input + controlled increments
                let s = std::str::from_utf8(&pds).unwrap();
                writeln!(out, "{s},N,")?;
            }
        }
        OutputFormat::Sql => {
            write!(out, "[")?;
            for i in 0..quantity {
                increment_pds(&mut pds)?;
                let s = std::str::from_utf8(&pds).unwrap();
                if i < quantity - 1 {
                    write!(out, "'{s}',")?;
                } else {
                    write!(out, "'{s}'")?;
                }
            }
            writeln!(out, "]")?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_simple() {
        let mut pds = parse_pds("W1UL5").unwrap();
        increment_pds(&mut pds).unwrap();
        assert_eq!(&pds, b"W1UL6");
    }

    #[test]
    fn test_9_to_a() {
        let mut pds = parse_pds("W1UL9").unwrap();
        increment_pds(&mut pds).unwrap();
        assert_eq!(&pds, b"W1ULA");
    }

    #[test]
    fn test_carry_z() {
        let mut pds = parse_pds("W1ULZ").unwrap();
        increment_pds(&mut pds).unwrap();
        assert_eq!(&pds, b"W1UM0");
    }

    #[test]
    fn test_overflow() {
        let mut pds = parse_pds("WZZZZ").unwrap();
        assert!(matches!(increment_pds(&mut pds), Err(PdsError::Overflow)));
    }

    #[test]
    fn test_invalid_length() {
        assert!(matches!(parse_pds("W1UL"), Err(PdsError::InvalidLength(4))));
    }

    #[test]
    fn test_invalid_prefix() {
        assert!(matches!(
            parse_pds("A1UL5"),
            Err(PdsError::InvalidPrefix(b'A'))
        ));
    }

    #[test]
    fn test_invalid_char() {
        assert!(matches!(
            parse_pds("W1UL!"),
            Err(PdsError::InvalidChar(b'!'))
        ));
    }
}
