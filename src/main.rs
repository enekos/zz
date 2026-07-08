use serde::Deserialize;
use std::env;
use std::io::{self, Write};
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
        "Usage: zz [-r|--root] [-w|--worktree <branch>] [-s|--search <term>] [--success-only] [-V|--version] <query>[@branch] [command...]"
    );
    eprintln!("\nOptions:");
    eprintln!(
        "  -r, --root              Execute command at the git root of the resolved directory"
    );
    eprintln!("  -w, --worktree <branch> Use a specific git worktree for the resolved repository");
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
        "  zz data@fix-foo ls          # Uses the worktree for branch 'fix-foo' in the 'data' repo"
    );
    eprintln!("  zz -w fix-foo data ls       # Same, using the explicit flag");
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
}

fn find_worktree_path(repo_root: &Path, branch: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root.as_os_str())
        .args(["worktree", "list", "--porcelain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_path: Option<PathBuf> = None;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            let branch_ref = rest;
            let branch_name = branch_ref.strip_prefix("refs/heads/").unwrap_or(branch_ref);
            if branch_name == branch {
                return current_path.clone();
            }
        }
    }

    None
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

    let final_dir = match branch {
        Some(branch) => {
            let repo_root = find_git_root(&target_dir);
            if !repo_root.join(".git").exists() {
                eprintln!(
                    "zz: worktree lookup requires a git repository; '{}' is not in one",
                    target_dir.display()
                );
                exit(1);
            }
            match find_worktree_path(&repo_root, &branch) {
                Some(wt_path) => wt_path,
                None => {
                    eprintln!(
                        "zz: no worktree found for branch '{}' in {}",
                        branch,
                        repo_root.display()
                    );
                    exit(1);
                }
            }
        }
        None => target_dir,
    };

    // Load env vars from the target directory before executing
    load_env_files(&final_dir);

    if cmd_args.is_empty() {
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
