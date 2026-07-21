# bash completion for zz — https://github.com/enekos/zz
# Install: source this file from your ~/.bashrc:
#   source /path/to/zz/completions/zz.bash
_zz() {
  local cur prev
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"

  case "$prev" in
    -w|--worktree)
      local branches
      branches=$( { wt ls 2>/dev/null | awk 'NR>1 && $1!~/^#/{print $1}'; \
                    git worktree list --porcelain 2>/dev/null \
                      | awk '/^branch /{sub(/^refs\/heads\//,"",$2);print $2}'; } | sort -u )
      COMPREPLY=( $(compgen -W "$branches" -- "$cur") )
      return
      ;;
    -s|--search)
      return
      ;;
  esac

  if [[ "$cur" == -* ]]; then
    COMPREPLY=( $(compgen -W \
      "-r --root -w --worktree -c --create -t --track -p --print -W --worktrees --json -s --search --success-only -V --version -h --help" \
      -- "$cur") )
    return
  fi

  # query@branch — complete branches of the repo the query resolves to
  if [[ "$cur" == *@* ]]; then
    local q="${cur%@*}" dir branches
    dir=$(zoxide query "$q" 2>/dev/null)
    if [[ -n "$dir" ]]; then
      branches=$(git -C "$dir" worktree list --porcelain 2>/dev/null \
        | awk '/^branch /{sub(/^refs\/heads\//,"",$2);print $2}')
      COMPREPLY=( $(compgen -P "$q@" -W "$branches" -- "${cur#*@}") )
    fi
    return
  fi

  # first positional: zoxide directory basenames
  local dirs
  dirs=$(zoxide query --list 2>/dev/null | awk -F/ '{print $NF}' | sort -u)
  COMPREPLY=( $(compgen -W "$dirs" -- "$cur") )
}
complete -F _zz zz
