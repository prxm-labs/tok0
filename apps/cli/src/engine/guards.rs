//! Command-class predicates used by both the runtime External arm
//! (to fail fast on commands that can't work through tok0) and the
//! PreToolUse rewriter (to pass these commands through unchanged so
//! the agent's shell handles them natively).
//!
//! Lives in `engine` rather than `main.rs` so the `bridge::rewriter`
//! crate-local module can call into it without a circular dep.

/// True for shell builtins whose effect is local to the shell process —
/// running them through tok0 puts them in a child process where they
/// either no-op (cd, export) or fail confusingly. Path-qualified names
/// (`/usr/bin/cd`, `./cd`) are real binaries, not the builtin.
pub fn is_shell_builtin(cmd: &str) -> bool {
    if cmd.is_empty() || cmd.contains('/') {
        return false;
    }
    matches!(
        cmd,
        "cd" | "pushd"
            | "popd"
            | "export"
            | "set"
            | "unset"
            | "source"
            | "."
            | "alias"
            | "unalias"
            | "eval"
            | "exec"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_detects_cwd_mutators() {
        for cmd in ["cd", "pushd", "popd"] {
            assert!(is_shell_builtin(cmd), "{cmd} should be detected");
        }
    }

    #[test]
    fn builtin_detects_env_mutators() {
        for cmd in ["export", "set", "unset"] {
            assert!(is_shell_builtin(cmd));
        }
    }

    #[test]
    fn builtin_detects_sourcing() {
        for cmd in ["source", "."] {
            assert!(is_shell_builtin(cmd));
        }
    }

    #[test]
    fn builtin_detects_alias_and_eval() {
        for cmd in ["alias", "unalias", "eval", "exec"] {
            assert!(is_shell_builtin(cmd));
        }
    }

    #[test]
    fn builtin_returns_false_for_real_commands() {
        for cmd in ["git", "cargo", "npm", "rg", "ls", "wc", "grep", "find"] {
            assert!(!is_shell_builtin(cmd));
        }
    }

    #[test]
    fn builtin_returns_false_for_empty_or_paths() {
        assert!(!is_shell_builtin(""));
        assert!(!is_shell_builtin("/usr/bin/cd"));
        assert!(!is_shell_builtin("./cd"));
    }
}
