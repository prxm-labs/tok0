# Amp (Sourcegraph)

Writes [`AGENTS.md`](AGENTS.md) into `~/.config/amp/`. Amp auto-loads it.

Amp walks up from cwd to `$HOME` loading every `AGENTS.md` it finds, plus `~/.config/amp/AGENTS.md` and `~/.config/AGENTS.md` globally. Verify pickup with `agents-md list`.

`tok0 init --uninstall` strips tok0 lines and leaves anything else alone.

Docs: <https://ampcode.com/manual>
