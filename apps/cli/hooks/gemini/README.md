# Gemini CLI

Writes [`GEMINI.md`](GEMINI.md) into `~/.gemini/`. Gemini auto-loads it.

Gemini concatenates this global file with any project-level `GEMINI.md` it finds while walking up from cwd.

`tok0 init --uninstall` removes the file if it only contains tok0 content; otherwise it strips the tok0 lines and leaves the rest.

Docs: <https://geminicli.com/docs/cli/gemini-md/>
