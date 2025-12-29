use artifacts::backend::helpers::{escape_single_quoted, fnv1a64, pretty_print_shell_escape};
use insta::assert_debug_snapshot;

// ---- Tests ----

#[test]
fn test_escape_single_quoted() {
    let cases = vec!["", "noquotes", "it's fine", "''''", "a'b"];
    let outputs: Vec<(String, String)> = cases
        .into_iter()
        .map(|inp| (inp.to_string(), escape_single_quoted(inp)))
        .collect();
    assert_debug_snapshot!("escape_single_quoted", outputs);
}

#[test]
fn test_pretty_print_shell_escape() {
    let cases = vec![
        "",
        "simple",
        "with space",
        "needs$var",
        "it's quoted",
        "already\"quoted\"",
        "path/with[brackets]",
        "no_specials",
    ];
    let outputs: Vec<(String, String)> = cases
        .into_iter()
        .map(|inp| (inp.to_string(), pretty_print_shell_escape(inp)))
        .collect();
    assert_debug_snapshot!("pretty_print_shell_escape", outputs);
}

#[test]
fn test_fnv1a64() {
    let cases = vec![
        "",
        "a",
        "hello",
        "Hello, world!",
        "/abs/path",
        "rel/path",
        "with spaces",
        "emoji 😀",
        "mix/With-CHARS_123",
    ];

    // Use hex to make the snapshot compact and stable across platforms
    let outputs: Vec<(String, String)> = cases
        .into_iter()
        .map(|inp| (inp.to_string(), format!("{:016x}", fnv1a64(inp))))
        .collect();

    insta::assert_debug_snapshot!("fnv1a64", outputs);
}
