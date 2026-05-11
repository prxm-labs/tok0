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

/// True when `cmd` looks like a POSIX variable assignment (`FOO=bar`,
/// `_X=1`) rather than a binary name. Agents sometimes write `tok0
/// FOO=bar cargo build`; clap parses `FOO=bar` as argv[0] and tok0
/// tries to spawn it as a program.
///
/// Path-qualified names (`./FOO=bar`) are treated as literal binaries.
pub fn is_var_assignment(cmd: &str) -> bool {
    let Some((name, _value)) = cmd.split_once('=') else {
        return false;
    };
    if name.contains('/') {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_assignment_detects_basic_form() {
        for cmd in ["FOO=bar", "_X=1", "DEBUG=true", "PATH=/usr/local/bin"] {
            assert!(is_var_assignment(cmd), "{cmd} should be detected");
        }
    }

    #[test]
    fn var_assignment_detects_empty_value() {
        assert!(is_var_assignment("FOO="));
    }

    #[test]
    fn var_assignment_rejects_flags_and_paths() {
        assert!(!is_var_assignment("--name=value"));
        assert!(!is_var_assignment("/path/to/FOO=bar"));
        assert!(!is_var_assignment("./FOO=bar"));
    }

    #[test]
    fn var_assignment_rejects_invalid_identifiers() {
        assert!(!is_var_assignment("=foo"));
        assert!(!is_var_assignment("123=foo"));
        assert!(!is_var_assignment("foo bar=baz"));
        assert!(!is_var_assignment("foo.bar=baz"));
    }

    #[test]
    fn var_assignment_rejects_plain_commands() {
        for cmd in ["git", "cargo", "npm", "echo", "cd"] {
            assert!(!is_var_assignment(cmd));
        }
    }

    #[test]
    fn var_assignment_rejects_empty_string() {
        assert!(!is_var_assignment(""));
    }

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
