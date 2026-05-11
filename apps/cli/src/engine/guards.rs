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

/// True for commands that drive a full-screen TUI or run an interactive
/// REPL. Running them through tok0 (which captures stdout/stderr with
/// piped stdio) breaks terminal rendering and password prompts. Caller
/// should bypass `run_proxy` and exec these with TTY inherited.
///
/// Path-qualified names (`/usr/bin/vim`, `./mycli`) are treated as
/// literal binaries — callers can wrap them explicitly if needed.
pub fn is_interactive_command(cmd: &str) -> bool {
    if cmd.is_empty() || cmd.contains('/') {
        return false;
    }
    // Restricted to commands that are *always* interactive/TUI when
    // invoked. Tools like `bun`, `deno`, `node`, `python`, `psql`, etc.
    // double as batch runners (`bun test`, `psql -c 'select 1'`); routing
    // them to TTY-inherit would skip compression on legitimate batch
    // workflows. Those dual-use cases stay on the run_proxy path.
    matches!(
        cmd,
        // Editors
        "vim" | "nvim" | "vi" | "nano" | "emacs" | "kak" | "helix" | "hx"
        // Pagers / docs
        | "less" | "more" | "man" | "info"
        // Process / system monitors
        | "top" | "htop" | "btop" | "atop" | "iotop"
        // File managers
        | "mc" | "ranger" | "nnn" | "lf"
        // Mail / chat clients
        | "mutt" | "neomutt" | "pine" | "alpine"
        // Remote shells / multiplexers
        | "ssh" | "telnet" | "screen" | "tmux" | "zellij" | "byobu"
        // Interactive pickers
        | "fzf" | "dialog" | "whiptail" | "gum"
        // Live watchers
        | "watch"
        // REPLs that have no useful batch mode
        | "irb" | "pry" | "ipython" | "ghci" | "ocaml"
    )
}

/// True for privilege-escalation wrappers (`sudo`, `doas`, `pkexec`,
/// `su`). These need direct TTY access for password prompts; piping
/// stdin/stdout through tok0 orphans the prompt and the user hangs.
pub fn is_privilege_escalator(cmd: &str) -> bool {
    if cmd.is_empty() || cmd.contains('/') {
        return false;
    }
    matches!(cmd, "sudo" | "doas" | "pkexec" | "su" | "runuser")
}

/// True when the command needs a real TTY — combines interactive UIs
/// and privilege escalators. Used by the External arm to bypass the
/// piped-stdio run_proxy path.
pub fn needs_tty(cmd: &str) -> bool {
    is_interactive_command(cmd) || is_privilege_escalator(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_detects_editors() {
        for cmd in ["vim", "nvim", "vi", "nano", "emacs"] {
            assert!(is_interactive_command(cmd));
        }
    }

    #[test]
    fn interactive_detects_pagers_and_monitors() {
        for cmd in ["less", "more", "man", "top", "htop", "btop"] {
            assert!(is_interactive_command(cmd));
        }
    }

    #[test]
    fn interactive_detects_remote_and_multiplexers() {
        for cmd in ["ssh", "telnet", "tmux", "screen", "zellij"] {
            assert!(is_interactive_command(cmd));
        }
    }

    #[test]
    fn interactive_detects_only_pure_repls() {
        // Only REPLs without a useful batch mode count. Dual-use tools
        // (psql, mysql, node, bun, deno, python) intentionally do NOT
        // — they're routinely scripted via -c / `run` / `script.py`.
        for cmd in ["irb", "pry", "ipython", "ghci", "ocaml"] {
            assert!(is_interactive_command(cmd), "{cmd} is a pure REPL");
        }
        for cmd in ["python", "node", "bun", "deno", "psql", "mysql"] {
            assert!(
                !is_interactive_command(cmd),
                "{cmd} doubles as a batch runner; must stay on run_proxy"
            );
        }
    }

    #[test]
    fn interactive_returns_false_for_batch_tools() {
        for cmd in ["git", "cargo", "rg", "grep", "ls", "find", "wc"] {
            assert!(!is_interactive_command(cmd));
        }
    }

    #[test]
    fn interactive_returns_false_for_empty_and_paths() {
        assert!(!is_interactive_command(""));
        assert!(!is_interactive_command("/usr/bin/vim"));
        assert!(!is_interactive_command("./vim"));
    }

    #[test]
    fn privilege_escalator_detects_common_wrappers() {
        for cmd in ["sudo", "doas", "pkexec", "su", "runuser"] {
            assert!(is_privilege_escalator(cmd));
        }
    }

    #[test]
    fn privilege_escalator_returns_false_for_regulars() {
        for cmd in ["git", "cargo", "vim", ""] {
            assert!(!is_privilege_escalator(cmd));
        }
    }

    #[test]
    fn needs_tty_is_union_of_both_sets() {
        assert!(needs_tty("vim"));
        assert!(needs_tty("sudo"));
        assert!(!needs_tty("git"));
    }

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
