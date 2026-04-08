//! Helper for the equivalence test script.
//! Reads EQUIV_COMPRESSOR and EQUIV_FIXTURE env vars, applies the
//! compressor, and prints the result between markers.
//!
//! Not a real test — used by scripts/equivalence_test.sh.

use tok0::compressors::git::git_cmd;
use tok0::compressors::rust::cargo_cmd;
use tok0::compressors::system::ls_cmd;

#[test]
fn compress_and_print() {
    let compressor = match std::env::var("EQUIV_COMPRESSOR") {
        Ok(v) => v,
        Err(_) => return, // No env var = running via `cargo test --all`, just pass
    };
    let fixture_path = match std::env::var("EQUIV_FIXTURE") {
        Ok(v) => v,
        Err(_) => return,
    };

    let input = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", fixture_path, e));

    let output = match compressor.as_str() {
        "ls" => ls_cmd::filter_ls(&input, "."),
        "git_status" | "git_status_count" => git_cmd::filter_git_status(&input),
        "git_log" => git_cmd::filter_git_log(&input),
        "git_diff" => git_cmd::filter_git_diff(&input),
        "cargo_test" => cargo_cmd::filter_cargo_test(&input),
        "cargo_build" => cargo_cmd::filter_cargo_build(&input),
        "cargo_clippy" => cargo_cmd::filter_cargo_clippy(&input),
        other => panic!("Unknown compressor: {}", other),
    };

    println!("__COMPRESSED_START__");
    print!("{}", output);
    if !output.ends_with('\n') {
        println!();
    }
    println!("__COMPRESSED_END__");
}
