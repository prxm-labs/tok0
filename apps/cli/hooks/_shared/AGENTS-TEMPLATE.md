# tok0 — Token-Optimized Command Proxy

This project uses tok0 to reduce AI token consumption. tok0 compresses
verbose command output before it reaches the LLM, typically saving
60–90% of tokens on git/cargo/npm/docker/kubectl and ~250 other commands.

## Usage

Prefix commands with `tok0` to route them through the proxy:

```
tok0 git status
tok0 cargo build
tok0 npm test
```

Run `tok0 stats` to see cumulative savings.

## Pipes, chains, and redirects

`tok0` is a single command. Shell operators (`|`, `&&`, `||`, `;`, `>`,
`<`, `&`) are handled by the shell, not by tok0. Use them normally:

```
tok0 git log --oneline | head -20
tok0 cargo test && tok0 cargo build --release
tok0 grep -r pattern src/ > matches.txt
```

When stdout is piped to another command (e.g. `tok0 find … | xargs …`),
tok0 passes output through verbatim. Compression only runs when stdout
is a TTY or the agent has set `TOK0_FORCE_COMPRESS=1`.

## Don't prefix these with tok0

Some commands can't run as a child process. Run them plain (without
`tok0`):

- **Directory navigation** — `cd`, `pushd`, `popd`. These mutate the
  parent shell's working directory; a child process can't do that.
  Write `cd path && tok0 cargo build` (plain `cd`, prefixed `cargo`).
- **Shell builtins** — `export`, `set`, `unset`, `source`, `.`,
  `alias`, `unalias`, `eval`, `exec`. They alter shell state that
  tok0's subprocess can't reach.
- **Shell control flow** — `if`, `for`, `while`, `case`, `[`, `[[`,
  `((`. These are shell keywords, not binaries; tok0 cannot wrap them.
- **Variable-assignment prefix** — `FOO=bar cmd`. tok0 sees `FOO=bar`
  as the command name and fails. Use `tok0 bash -c 'FOO=bar cmd'`
  instead, or export the variable in your session first.
- **Privilege escalators** — `sudo`, `doas`, `pkexec`, `su`. These need
  a direct TTY for the password prompt; running them through tok0
  breaks the prompt.
- **Interactive / TUI tools** — `vim`, `nvim`, `nano`, `less`, `more`,
  `top`, `htop`, `ssh`, `psql -i`, `mysql`, `redis-cli`, `man`, `fzf`.
  tok0 captures stdout/stderr and would break their terminal rendering.

If you do prefix one of these by accident, tok0 fails fast with a
stderr message naming the offender — drop the `tok0` prefix and the
command works normally.
