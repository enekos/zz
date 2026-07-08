# zz

A tiny Rust CLI that combines [`zoxide`](https://github.com/ajeetdsouza/zoxide) with command execution, so you can run commands in the right directory without typing the full path.

```bash
zz proj cargo test
zz data pnpm dev
zz -w fix-foo data pnpm test   # jump to a git worktree
cd "$(zz -p data@fix-foo)"     # -p prints the resolved path, runs nothing
```

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [`zoxide`](https://github.com/ajeetdsouza/zoxide) — required for directory resolution
- (Optional) [`fzf`](https://github.com/junegunn/fzf) — for interactive command history
- (Optional) `aztarna` — for project-specific command suggestions (falls back to a shell if unavailable)

### From source

```bash
git clone https://github.com/enekos/zz.git
cd zz
cargo install --path .
```

Or use the provided install script:

```bash
./install.sh
```

The install script builds the release binary, copies it to `~/.local/bin`, and installs the zsh completion. If you had a `compdef ... zz` line binding `zz` to zoxide's completer, remove it — it overrides the bundled `_zz`.

### Shell completions

`zz` ships completions that complete directory queries, `@branch` / `-w` branch names (from `git worktree list` and `wt`), and flags.

- **zsh**: put `completions/_zz` on your `$fpath` (the install script drops it in `~/.oh-my-zsh/custom/completions`), then re-run `compinit`.
- **bash**: `source completions/zz.bash` from your `~/.bashrc`.

## Usage

```bash
zz [-r|--root] [-w|--worktree <branch>] [-c|--create] [-p|--print] [-s|--search <term>] [--success-only] <query>[@branch] [command...]
```

- `<query>` is passed straight to `zoxide query` to resolve the best-matching directory.
- `[command...]` is the command and its arguments to run in that directory.
- `-r`, `--root` — walk up to the `.git` root of the resolved directory before running the command.
- `-w <branch>`, `--worktree <branch>` — resolve the repository, then switch to the git worktree for that branch before running the command.
- `<query>@<branch>` — shorthand for the above (`zz data@fix-foo pnpm test`).
- `<query>@?` — interactive `fzf` picker over that repo's worktrees (terminal only).
- `-c`, `--create` — if the requested worktree is missing, create it by delegating to `wt new <branch>` (service guessed from the repo dir name), then run there. Without `-c`, a missing worktree prompts y/N in a terminal, or errors with the exact `wt new` command otherwise.
- `-p`, `--print` — print the fully-resolved directory and exit without running anything. Composes: `cd "$(zz -p data@fix)"`, `git -C "$(zz -p data)" log`.
- `-s <term>`, `--search <term>` — in interactive mode, pre-filter `aztarna` suggestions to commands containing `<term>`.
- `--success-only` — in interactive mode, only show `aztarna` commands that exited successfully.
- `-V`, `--version` — print version.

When no command is given **and stdin is a terminal**, `zz` shows recent commands for that directory via `aztarna` + `fzf`, dropping you into an interactive shell if you don't pick one. When stdin is **not** a terminal (e.g. an agent or a script), it prints the resolved path and exits — it never blocks on `fzf` or a shell.

## Examples

```bash
# Run tests in the project zoxide knows as "aztarna"
zz azt cargo test

# Start a dev server at the repo root even if zoxide resolved to a nested folder
zz -r meta pnpm dev

# Jump to a specific worktree and run tests there
zz data@fix-foo pnpm test
zz -w fix-foo data pnpm test

# Pick a worktree interactively, then create one if it's missing
zz data@? pnpm test
zz -c -w new-feature data pnpm dev:local

# Print the resolved path and compose with other tools (runs nothing)
cd "$(zz -p data@fix-foo)"
git -C "$(zz -p data)" status

# Interactive picker, pre-filtered to commands containing "test"
zz -s test data

# Interactive picker, only commands that exited successfully
zz --success-only data

# Read a file in dotfiles
zz dot cat README.md

# No command — interactive command history (terminal) or just the path (non-tty)
zz proj
```

## Environment

`zz` loads `.env` and `.env.local` from the target directory before executing the command.

## License

MIT
