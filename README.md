# zz

A tiny Rust CLI that combines [`zoxide`](https://github.com/ajeetdsouza/zoxide) with command execution, so you can run commands in the right directory without typing the full path.

```bash
zz proj cargo test
zz data pnpm dev
zz -w fix-foo data pnpm test   # jump to a git worktree
zz data@shared pnpm test       # partial branch match — no need for the full name
zz -W --json data              # list the repo's worktrees (agent-safe, JSON)
cd "$(zz -p data@fix-foo)"     # -p prints the resolved path, runs nothing
```

## Installation

### One line (prebuilt binary — no Rust needed)

```bash
curl -fsSL https://raw.githubusercontent.com/enekos/zz/main/install.sh | bash
```

This downloads the right prebuilt binary for your platform from the latest
[release](https://github.com/enekos/zz/releases), drops it in `~/.local/bin`, and
installs shell completions. Supported: macOS (Apple Silicon & Intel) and Linux
(x86_64 & arm64, static musl).

Overrides: `INSTALL_DIR` (install location), `ZZ_VERSION` (pin a tag like `v0.3.0`),
`ZSH_COMPLETION_DIR` (completion location).

### Linux packages

```bash
yay -S zz-cli                                     # Arch, from the AUR
sudo apt install ./zz-cli_<version>_amd64.deb       # Debian, Ubuntu
sudo dnf install ./zz-cli-<version>-1.x86_64.rpm    # Fedora, RHEL
sudo apk add --allow-untrusted ./zz-cli_<version>_x86_64.apk   # Alpine
nix run github:enekos/zz                     # Nix
```

The `.deb`, `.rpm` and `.apk` files are on each [release](https://github.com/enekos/zz/releases), for x86_64 and arm64.

### From crates.io

```bash
cargo install zz-cli
```

The crate is `zz-cli`; the binary it installs is `zz`. This route builds from source and does
not install shell completions — see below for those.

### Prerequisites

- [`zoxide`](https://github.com/ajeetdsouza/zoxide) — required for directory resolution
- (Optional) [`fzf`](https://github.com/junegunn/fzf) — for interactive command history
- (Optional) `aztarna` — for project-specific command suggestions (falls back to a shell if unavailable)

### From source

Needs a [Rust toolchain](https://www.rust-lang.org/tools/install):

```bash
git clone https://github.com/enekos/zz.git
cd zz
cargo install --path .
```

If you had a `compdef ... zz` line binding `zz` to zoxide's completer, remove it — it overrides the bundled `_zz`.

### Shell completions

`zz` ships completions that complete directory queries, `@branch` / `-w` branch names (from `git worktree list` and `wt`), and flags.

- **zsh**: put `completions/_zz` on your `$fpath` (the install script drops it in `~/.oh-my-zsh/custom/completions`), then re-run `compinit`.
- **bash**: `source completions/zz.bash` from your `~/.bashrc`.

## Usage

```bash
zz [-r|--root] [-w|--worktree <branch>] [-c|--create] [-t|--track] [-p|--print] [-W|--worktrees] [--json] [-s|--search <term>] [--success-only] <query>[@branch] [command...]
```

- `<query>` is passed straight to `zoxide query` to resolve the best-matching directory.
- **Worktree discovery fallback** — when the plain query matches nothing in zoxide, `zz` scans the linked worktrees of every repo zoxide knows and matches branch and worktree-dir names (exact → unique prefix → unique substring). So `zz fix-foo` finds the `fix-foo` worktree even if you've never visited it. Ambiguous fragments list the candidates and exit non-zero instead of guessing.
- **Frecency learning** — worktrees `zz` lands in are taught back to zoxide (`zoxide add`), so they climb the rankings and plain `z`/`zoxide query` learn them over time.
- `[command...]` is the command and its arguments to run in that directory.
- `-r`, `--root` — walk up to the `.git` root of the resolved directory before running the command.
- `-w <branch>`, `--worktree <branch>` — resolve the repository, then switch to the git worktree for that branch before running the command.
- `<query>@<branch>` — shorthand for the above (`zz data@fix-foo pnpm test`).
- **Partial branch matching** — the branch in `@branch` / `-w` need not be exact. `zz` resolves it most-specific-first: exact name → unique prefix → unique substring. An exact name always wins. If the fragment matches more than one worktree, `zz` lists the candidates and exits non-zero instead of guessing (`zz data@shared` → the sole `shared*` worktree).
- `-W`, `--worktrees` — list the resolved repo's worktrees as `branch<TAB>path` and exit. Never spawns `fzf` or a shell, so it's safe from a script or agent. Add `--json` to emit `[{"branch": "...", "path": "..."}]`.
- `<query>@?` — interactive `fzf` picker over that repo's worktrees (terminal only).
- `-c`, `--create` — if the requested worktree is missing, create it with plain `git worktree add` (no external helper) into a sibling `<repo>-worktrees/<branch-slug>` dir, then run there. An existing branch is checked out; a new branch is cut from the base ref (`main` → `master` → current `HEAD`). Without `-c`, a missing worktree prompts y/N in a terminal, or errors with the path it would create otherwise. Set `ZZ_WORKTREE_DIR` to relocate worktrees (grouped by repo name) and `ZZ_WORKTREE_BASE` to change the base ref for new branches.
- `-t`, `--track` — pull a worktree for a **remote** branch: the fragment is matched against remote branch names (fresh `git ls-remote` per remote, same partial-match tiers), the unique match is fetched, and the worktree is created with `git worktree add --track -b <branch> <path> <remote>/<branch>` in the same sibling location as `-c`. If the worktree or local branch already exists, it's reused. When several remotes carry the branch, `origin` wins.
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

# Don't know the exact branch? A unique fragment resolves it
zz data@shared pnpm test

# List a repo's worktrees (agent/script-safe — no fzf, no shell)
zz -W data
zz -W --json data

# Pick a worktree interactively, then create one if it's missing
zz data@? pnpm test
zz -c -w new-feature data pnpm dev:local

# Pull a worktree for a branch that only exists on the remote
zz -t data@feat-csrf pnpm test

# Jump to a worktree you've never visited — zoxide fallback finds it
zz fix-foo pnpm test

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
