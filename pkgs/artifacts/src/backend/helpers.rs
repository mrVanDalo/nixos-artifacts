use std::path::{Path, PathBuf};

// Compute a deterministic filename based on the 'out' path to keep test snapshots stable
pub fn fnv1a64(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    const PRIME: u64 = 0x00000100000001B3; // FNV prime
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

pub(crate) fn resolve_path(base_dir: &Path, relative_path: &str) -> PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[rustfmt::skip]
pub fn pretty_print_shell_escape(input: &str) -> String {
    let needs_quotes = input.is_empty() || input.chars().any(|character| { character.is_whitespace() || matches!( character, '\'' | '"' | '\\' | '$' | '&' | '|' | ';' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' ) });
    if needs_quotes {
        format!("'{}'", escape_single_quoted(input))
    } else {
        input.to_string()
    }
}

// Replace ' with '\'' for safe single-quoting
pub fn escape_single_quoted(input: &str) -> String {
    input.replace('\'', "'\\''")
}
