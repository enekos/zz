# zz

A tiny Rust CLI that combines [`zoxide`](https://github.com/ajeetdsouza/zoxide) with command execution, so you can run commands in the right directory without typing the full path.

```bash
zz proj cargo test
zz data pnpm dev
zz -w fix-foo data pnpm test   # jump to a git worktree
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

The install script builds the release binary and copies it to `~/.local/bin`.

## Usage

```bash
zz [-r|--root] [-w|--worktree <branch>] [-s|--search <term>] [--success-only] <query>[@branch] [command...]
```

- `<query>` is passed straight to `zoxide query` to resolve the best-matching directory.
- `[command...]` is the command and its arguments to run in that directory.
- `-r`, `--root` — walk up to the `.git` root of the resolved directory before running the command.
- `-w <branch>`, `--worktree <branch>` — resolve the repository, then switch to the git worktree for that branch before running the command.
- `<query>@<branch>` — shorthand for the above (`zz data@fix-foo pnpm test`).
- `-s <term>`, `--search <term>` — in interactive mode, pre-filter `aztarna` suggestions to commands containing `<term>`.
- `--success-only` — in interactive mode, only show `aztarna` commands that exited successfully.
- `-V`, `--version` — print version.

When no command is given, `zz` tries to show recent commands for that directory via `aztarna` + `fzf`; if that's not available, it drops you into an interactive shell.

## Examples

```bash
# Run tests in the project zoxide knows as "aztarna"
zz azt cargo test

# Start a dev server at the repo root even if zoxide resolved to a nested folder
zz -r meta pnpm dev

# Jump to a specific worktree and run tests there
zz data@fix-foo pnpm test
zz -w fix-foo data pnpm test

# Interactive picker, pre-filtered to commands containing "test"
zz -s test data

# Interactive picker, only commands that exited successfully
zz --success-only data

# Read a file in dotfiles
zz dot cat README.md

# No command — interactive command history or new shell
zz proj
```

## Environment

`zz` loads `.env` and `.env.local` from the target directory before executing the command.

## License

MIT
