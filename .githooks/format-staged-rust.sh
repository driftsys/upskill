#!/usr/bin/env bash
set -euo pipefail

staged_rust_files=()
while IFS= read -r -d '' file; do
  case "$file" in
    *.rs) staged_rust_files+=("$file") ;;
  esac
done < <(git diff --cached --name-only -z --diff-filter=ACMRT)

if ((${#staged_rust_files[@]} > 0)); then
  cargo fmt -- "${staged_rust_files[@]}"
fi
