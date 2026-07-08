use serde::Deserialize;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};

#[derive(Deserialize)]
struct TopRow {
    last_command: String,
}

fn print_usage() {
    eprintln!("zz - Run commands in directories via zoxide");
    eprintln!(
        "Usage: zz [-r|--root] [-w|--worktree <branch>] [-c|--create] [-p|--print]\n          [-s|--search <term>] [--success-only] [-V|--version] <query>[@branch] [command...]"
    );
    eprintln!("\nOptions:");
    eprintln!(
        "  -r, --root              Execute command at the git root of the resolved directory"
    );
    eprintln!("  -w, --worktree <branch> Use a specific git worktree for the resolved repository");
    eprintln!(
        "  -c, --create            If the worktree is missing, create it via `wt new <branch>`"
    );
    eprintln!(
        "  -p, --print             Print the fully-resolved directory and exit (run nothing)"
    );
    eprintln!(
        "  -s, --search <term>     Pre-filter aztarna suggestions by command text (interactive mode)"
    );
    eprintln!("  --success-only          Only show aztarna commands that exited successfully");
    eprintln!("  -V, --version           Print version");
    eprintln!("\nExamples:");
    eprintln!("  zz data ls -al              # Runs 'ls -al' in the best match for 'data'");
    eprintln!("  zz meta npm test            # Runs 'npm test' in the best match for 'meta'");
    eprintln!(
        "  zz -r proj cargo t          # Resolves 'proj', walks up to git root, runs 'cargo t'"
    );
    eprintln!(
        "  zz -p data@fix-foo          # Prints the worktree path (compose: cd \"$(zz -p ...)\")"
    );
    eprintln!(
        "  zz data@fix-foo ls          # Uses the worktree for branch 'fix-foo' in the 'data' repo"
    );
    eprintln!(
        "  zz data@?                   # Interactive picker over the 'data' repo's worktrees"
    );
    eprintln!("  zz -w fix-foo data ls       # Same, using the explicit flag");
    eprintln!(
        "  zz -c -w new-br data ls     # Create the worktree via `wt new` first if it's missing"
    );
    eprintln!("  zz -s test data             # Interactive picker pre-filtered to 'test' commands");
    eprintln!(
        "  zz --success-only data      # Interactive picker showing only commands that usually succeed"
    );
    eprintln!(
        "  zz proj                     # Uses aztarna + fzf to pick a previous command, or drops to shell"
    );
    exit(1);
}

fn find_git_root(start: &Path) -> PathBuf {
    let mut current = start;
    loop {
        if current.join(".git").exists() {
            return current.to_path_buf();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return start.to_path_buf(), // Fallback to start if no .git found
        }
    }
}

fn split_query_branch(raw: &str) -> (String, Option<String>) {
    if let Some(pos) = raw.rfind('@') {
        let (query, branch) = raw.split_at(pos);
        (query.to_string(), Some(branch[1..].to_string()))
    } else {
        (raw.to_string(), None)
    }
}

/// Parse the output of `git worktree list --porcelain` into (branch, path) pairs.
/// Detached-HEAD worktrees (no `branch` line) are skipped.
fn parse_worktree_porcelain(text: &str) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut current_path: Option<PathBuf> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            let branch_name = rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string();
            if let Some(path) = current_path.take() {
                out.push((branch_name, path));
            }
        } else if line.is_empty() {
            current_path = None;
        }
    }

    out
}

fn list_worktrees(repo_root: &Path) -> Vec<(String, PathBuf)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_os_str())
        .args(["worktree", "list", "--porcelain"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            parse_worktree_porcelain(&String::from_utf8_lossy(&out.stdout))
        }
        _ => Vec::new(),
    }
}

fn find_worktree_path(repo_root: &Path, branch: &str) -> Option<PathBuf> {
    list_worktrees(repo_root)
        .into_iter()
        .find(|(b, _)| b == branch)
        .map(|(_, path)| path)
}

/// Interactive fzf picker over a repo's worktrees. Shows the branch, returns its path.
fn pick_worktree_fzf(repo_root: &Path) -> Option<PathBuf> {
    let worktrees = list_worktrees(repo_root);
    if worktrees.is_empty() {
        return None;
    }

    let mut fzf_input = String::new();
    for (branch, path) in &worktrees {
        // "branch\tpath" — fzf displays only the branch, we carry the path through.
        fzf_input.push_str(&format!("{}\t{}\n", branch, path.display()));
    }

    let mut fzf = Command::new("fzf")
        .args([
            "--height=40%",
            "--reverse",
            "--with-nth=1",
            "--delimiter=\t",
            "--prompt",
            "worktree > ",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = fzf.stdin.take() {
        let _ = stdin.write_all(fzf_input.as_bytes());
    }

    let result = fzf.wait_with_output().ok()?;
    if !result.status.success() {
        return None;
    }

    let selected = String::from_utf8_lossy(&result.stdout);
    let line = selected.lines().next()?;
    line.split('\t').nth(1).map(PathBuf::from)
}

fn prompt_yes_no(question: &str) -> bool {
    eprint!("{} [y/N] ", question);
    let _ = io::stderr().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Delegate worktree creation to `wt` (the lifecycle owner) and re-resolve the path.
/// The service is guessed from the repo directory name; `wt` reports if that's wrong.
fn try_create_worktree(repo_root: &Path, branch: &str) -> Option<PathBuf> {
    let svc = repo_root.file_name()?.to_string_lossy().to_string();
    eprintln!("zz: running `wt new {} -s {}`", branch, svc);

    let status = Command::new("wt")
        .args(["new", branch, "-s", &svc])
        .status();

    match status {
        Ok(s) if s.success() => find_worktree_path(repo_root, branch),
        Ok(_) => {
            eprintln!("zz: `wt new {}` failed", branch);
            None
        }
        Err(e) => {
            eprintln!(
                "zz: could not run `wt` ({}); is it installed and on PATH?",
                e
            );
            None
        }
    }
}

fn load_env_files(dir: &Path) {
    // Attempt to load .env and .env.local in the target directory
    // We ignore errors since the files might not exist or be readable.
    let _ = dotenvy::from_filename_override(dir.join(".env"));
    let _ = dotenvy::from_filename_override(dir.join(".env.local"));
}

fn interactive_aztarna_fzf(
    target_dir: &Path,
    search_term: Option<&str>,
    success_only: bool,
) -> Option<String> {
    let target_dir_str = target_dir.to_string_lossy();

    // Call aztarna top --json --cwd <dir> [--query <term>] [--success-only]
    let mut cmd = Command::new("aztarna");
    cmd.arg("top")
        .arg("--json")
        .arg("--cwd")
        .arg(&*target_dir_str);
    if let Some(term) = search_term {
        cmd.arg("--query").arg(term);
    }
    if success_only {
        cmd.arg("--success-only");
    }

    let output = cmd.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let rows: Vec<TopRow> = serde_json::from_slice(&output.stdout).ok()?;
    if rows.is_empty() {
        return None; // no commands logged yet
    }

    // Prepare inputs for fzf
    let mut fzf_input = String::new();
    for row in rows {
        fzf_input.push_str(&row.last_command);
        fzf_input.push('\n');
    }

    let mut fzf = Command::new("fzf")
        .arg("--height=40%")
        .arg("--reverse")
        .arg("--prompt")
        .arg(format!(
            "{} > ",
            target_dir.file_name().unwrap_or_default().to_string_lossy()
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = fzf.stdin.take() {
        let _ = stdin.write_all(fzf_input.as_bytes());
    }

    let result = fzf.wait_with_output().ok()?;
    if result.status.success() {
        let selected = String::from_utf8_lossy(&result.stdout).trim().to_string();
        if !selected.is_empty() {
            return Some(selected);
        }
    }

    None
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
    }

    let mut use_git_root = false;
    let mut worktree_branch: Option<String> = None;
    let mut create_worktree = false;
    let mut print_only = false;
    let mut search_term: Option<String> = None;
    let mut success_only = false;
    let mut positional: Vec<String> = Vec::new();
    let mut parsing_flags = true;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if parsing_flags && arg.starts_with('-') {
            match arg.as_str() {
                "-r" | "--root" => use_git_root = true,
                "-w" | "--worktree" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("zz: --worktree requires a branch name");
                        exit(1);
                    }
                    worktree_branch = Some(args[i].clone());
                }
                "-c" | "--create" => create_worktree = true,
                "-p" | "--print" => print_only = true,
                "-s" | "--search" => {
                    i += 1;
                    if i >= args.len() {
                        eprintln!("zz: --search requires a search term");
                        exit(1);
                    }
                    search_term = Some(args[i].clone());
                }
                "--success-only" => success_only = true,
                "-h" | "--help" => print_usage(),
                "-V" | "--version" => {
                    println!("zz {}", env!("CARGO_PKG_VERSION"));
                    exit(0);
                }
                _ => {
                    eprintln!("zz: unknown flag '{}' (try: zz --help)", arg);
                    exit(1);
                }
            }
        } else {
            parsing_flags = false;
            positional.push(arg.clone());
        }
        i += 1;
    }

    if positional.is_empty() {
        print_usage();
    }

    let raw_query = &positional[0];
    let cmd_args = &positional[1..];

    let (query, branch_from_query) = split_query_branch(raw_query);
    let branch = worktree_branch.or(branch_from_query);

    // Use zoxide to find the target directory
    let output = Command::new("zoxide").arg("query").arg(&query).output();

    let output = match output {
        Ok(out) => out,
        Err(e) => {
            eprintln!("zz: failed to execute 'zoxide': {}", e);
            eprintln!("Is zoxide installed and in your PATH?");
            exit(1);
        }
    };

    if !output.status.success() {
        io::stderr().write_all(&output.stderr).unwrap();
        exit(output.status.code().unwrap_or(1));
    }

    let resolved_path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let target_dir = if use_git_root {
        find_git_root(&resolved_path)
    } else {
        resolved_path
    };

    let final_dir = match branch.as_deref() {
        Some(branch_name) => {
            let repo_root = find_git_root(&target_dir);
            if !repo_root.join(".git").exists() {
                eprintln!(
                    "zz: worktree lookup requires a git repository; '{}' is not in one",
                    target_dir.display()
                );
                exit(1);
            }

            if branch_name == "?" {
                // Interactive worktree picker. Needs a terminal for fzf.
                if !io::stdin().is_terminal() {
                    eprintln!(
                        "zz: '@?' worktree picker requires a terminal; pass an explicit branch"
                    );
                    exit(1);
                }
                match pick_worktree_fzf(&repo_root) {
                    Some(path) => path,
                    None => {
                        eprintln!("zz: no worktree selected in {}", repo_root.display());
                        exit(1);
                    }
                }
            } else {
                match find_worktree_path(&repo_root, branch_name) {
                    Some(wt_path) => wt_path,
                    None => {
                        let svc = repo_root
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        // Create only on explicit --create, or an interactive yes.
                        let should_create = create_worktree
                            || (io::stdin().is_terminal()
                                && prompt_yes_no(&format!(
                                    "zz: no worktree for '{}'. create it with `wt new {} -s {}`?",
                                    branch_name, branch_name, svc
                                )));
                        if should_create {
                            match try_create_worktree(&repo_root, branch_name) {
                                Some(path) => path,
                                None => exit(1),
                            }
                        } else {
                            eprintln!(
                                "zz: no worktree for branch '{}' in {}",
                                branch_name,
                                repo_root.display()
                            );
                            eprintln!("     create it with:  wt new {} -s {}", branch_name, svc);
                            eprintln!("     or re-run with:  -c / --create");
                            exit(1);
                        }
                    }
                }
            }
        }
        None => target_dir,
    };

    // --print: resolve only. Nothing is run and no env is touched — usable in `$(...)`.
    if print_only {
        println!("{}", final_dir.display());
        exit(0);
    }

    // Load env vars from the target directory before executing
    load_env_files(&final_dir);

    if cmd_args.is_empty() {
        // No command given. In a non-interactive context (e.g. an agent), never
        // spawn fzf or an interactive shell — that would hang. Print the path.
        if !io::stdin().is_terminal() {
            println!("{}", final_dir.display());
            exit(0);
        }

        // Try interactive aztarna + fzf selection first
        if let Some(selected_cmd) =
            interactive_aztarna_fzf(&final_dir, search_term.as_deref(), success_only)
        {
            println!("zz: running `{}` in {}", selected_cmd, final_dir.display());

            // Execute the selected command by passing it to the shell
            let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
            let err = Command::new(&shell)
                .arg("-c")
                .arg(&selected_cmd)
                .current_dir(&final_dir)
                .exec();

            eprintln!("zz: failed to execute '{}': {}", selected_cmd, err);
            exit(1);
        } else {
            // Drop to shell if no aztarna commands, fzf not found, or user hit Escape
            let shell = env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
            println!("zz: starting {} in {}", shell, final_dir.display());
            let err = Command::new(&shell).current_dir(&final_dir).exec();
            eprintln!("zz: failed to execute shell '{}': {}", shell, err);
            exit(1);
        }
    } else {
        let cmd = &cmd_args[0];
        let exec_args = &cmd_args[1..];

        let err = Command::new(cmd)
            .args(exec_args)
            .current_dir(&final_dir)
            .exec();

        eprintln!("zz: failed to execute '{}': {}", cmd, err);
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_query_branch_without_at() {
        assert_eq!(split_query_branch("data"), ("data".to_string(), None));
    }

    #[test]
    fn split_query_branch_with_at() {
        assert_eq!(
            split_query_branch("data@fix-foo"),
            ("data".to_string(), Some("fix-foo".to_string()))
        );
    }

    #[test]
    fn split_query_branch_with_multiple_at() {
        assert_eq!(
            split_query_branch("data@fix@foo"),
            ("data@fix".to_string(), Some("foo".to_string()))
        );
    }

    #[test]
    fn split_query_branch_empty_query() {
        assert_eq!(
            split_query_branch("@main"),
            ("".to_string(), Some("main".to_string()))
        );
    }

    #[test]
    fn parse_worktree_porcelain_extracts_branch_and_path() {
        let text = "worktree /repo\nHEAD abc123\nbranch refs/heads/main\n\n\
                    worktree /repo/wt/fix-foo\nHEAD def456\nbranch refs/heads/fix-foo\n";
        assert_eq!(
            parse_worktree_porcelain(text),
            vec![
                ("main".to_string(), PathBuf::from("/repo")),
                ("fix-foo".to_string(), PathBuf::from("/repo/wt/fix-foo")),
            ]
        );
    }

    #[test]
    fn parse_worktree_porcelain_skips_detached() {
        let text = "worktree /repo\nHEAD abc123\ndetached\n";
        assert!(parse_worktree_porcelain(text).is_empty());
    }
}
