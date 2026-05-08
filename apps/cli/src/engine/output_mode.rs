//! Decides whether stdout should be compressed (TTY/agent context) or
//! passed through raw (shell pipeline).
//!
//! The find/ls/grep compressors are *lossy* by design — they group, count,
//! and truncate. That's correct for human and LLM consumers, but breaks
//! pipelines (`tok0 find … | xargs grep …`) because downstream tools need
//! exact, complete data. This module centralises the TTY-aware decision
//! so every compressor inherits the right behaviour through `run_proxy`.
//!
//! Default: compress when stdout is a TTY, pass through when piped — the
//! standard Unix convention (cf. `ls --color=auto`, `git --color=auto`).
//!
//! Overrides (highest precedence first):
//!   • `TOK0_FORCE_RAW=1`      → always raw, even in a TTY
//!   • `TOK0_FORCE_COMPRESS=1` → always compress, even in a pipe
//!     (use this from agent hooks that capture stdout for LLM context)
use std::io::IsTerminal;

/// Pure decision function — exposed for unit testing.
///
/// Precedence: `force_raw` > `force_compress` > TTY status.
fn decide(is_tty: bool, force_compress: bool, force_raw: bool) -> bool {
    if force_raw {
        return false;
    }
    if force_compress {
        return true;
    }
    is_tty
}

/// Read an env var as a "truthy" flag. Treats "0", "false", "no", and the
/// empty string as not-set so that `TOK0_FORCE_COMPRESS=0` does what users
/// expect.
fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !v.is_empty() && v != "0" && v != "false" && v != "no"
        }
        Err(_) => false,
    }
}

/// True when `run_proxy` should run command output through the compressor
/// pipeline + sanitiser. False means pass the raw command output through.
pub fn should_compress() -> bool {
    decide(
        std::io::stdout().is_terminal(),
        env_flag("TOK0_FORCE_COMPRESS"),
        env_flag("TOK0_FORCE_RAW"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise env-var tests across the suite — `std::env` is process-wide.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // ─────────────────────────────────────────────────────────────────
    // Pure decision-function truth table
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn decide_tty_no_overrides_compresses() {
        assert!(decide(true, false, false));
    }

    #[test]
    fn decide_pipe_no_overrides_passes_raw() {
        assert!(!decide(false, false, false));
    }

    #[test]
    fn decide_force_compress_overrides_pipe() {
        assert!(decide(false, true, false));
    }

    #[test]
    fn decide_force_raw_overrides_tty() {
        assert!(!decide(true, false, true));
    }

    #[test]
    fn decide_force_raw_wins_over_force_compress() {
        // Both set — raw wins (safer default: never destroy data
        // when the user has explicitly asked for raw).
        assert!(!decide(true, true, true));
        assert!(!decide(false, true, true));
    }

    // ─────────────────────────────────────────────────────────────────
    // env_flag parsing
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn env_flag_unset_is_false() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        std::env::remove_var("TOK0_TEST_FLAG_UNSET");
        assert!(!env_flag("TOK0_TEST_FLAG_UNSET"));
    }

    #[test]
    fn env_flag_truthy_values() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        for v in ["1", "true", "yes", "on", "TRUE", "  yes  "] {
            std::env::set_var("TOK0_TEST_FLAG_TRUTHY", v);
            assert!(env_flag("TOK0_TEST_FLAG_TRUTHY"), "{v:?} should be truthy");
        }
        std::env::remove_var("TOK0_TEST_FLAG_TRUTHY");
    }

    #[test]
    fn env_flag_falsy_values() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        for v in ["", "0", "false", "no", "FALSE", "  0  "] {
            std::env::set_var("TOK0_TEST_FLAG_FALSY", v);
            assert!(!env_flag("TOK0_TEST_FLAG_FALSY"), "{v:?} should be falsy");
        }
        std::env::remove_var("TOK0_TEST_FLAG_FALSY");
    }

    // ─────────────────────────────────────────────────────────────────
    // should_compress() integration with env vars (uses real stdout TTY)
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn should_compress_force_raw_disables_compression() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        std::env::remove_var("TOK0_FORCE_COMPRESS");
        std::env::set_var("TOK0_FORCE_RAW", "1");
        assert!(!should_compress());
        std::env::remove_var("TOK0_FORCE_RAW");
    }

    #[test]
    fn should_compress_force_compress_enables_compression() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        std::env::remove_var("TOK0_FORCE_RAW");
        std::env::set_var("TOK0_FORCE_COMPRESS", "1");
        assert!(should_compress());
        std::env::remove_var("TOK0_FORCE_COMPRESS");
    }

    #[test]
    fn should_compress_force_raw_beats_force_compress() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        std::env::set_var("TOK0_FORCE_COMPRESS", "1");
        std::env::set_var("TOK0_FORCE_RAW", "1");
        assert!(!should_compress());
        std::env::remove_var("TOK0_FORCE_COMPRESS");
        std::env::remove_var("TOK0_FORCE_RAW");
    }

    /// The cargo-test harness pipes stdout into its capture buffer, so
    /// `is_terminal()` is false here. With no overrides, the function must
    /// return `false` — the pipeline-friendly default. This is exactly the
    /// behaviour that fixes `tok0 find … | xargs …`.
    #[test]
    fn should_compress_default_in_test_harness_is_raw() {
        let _g = ENV_LOCK.lock().expect("env lock poisoned");
        std::env::remove_var("TOK0_FORCE_COMPRESS");
        std::env::remove_var("TOK0_FORCE_RAW");
        assert!(!should_compress(), "test stdout is a pipe → must be raw");
    }
}
