#!/usr/bin/env bash
# Fails unless every file that differs from the upstream base is listed in upstream-patches.md,
# and every listed path still differs.
# usage: scripts/fork-delta.sh [upstream ttu-ttu/ebook-reader clone]
set -euo pipefail

base=68c40f1ca934e8d743395ca0f08f44d7292c5683
fork=$(cd "$(dirname "$0")/.." && pwd)
upstream=${1:-${TTU_UPSTREAM:-}}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

if [[ -z $upstream ]]; then
  upstream=$work/clone
  git clone -q --filter=blob:none --no-checkout https://github.com/ttu-ttu/ebook-reader.git "$upstream"
fi
mkdir "$work/base"
git -C "$upstream" archive "$base" | tar -x -C "$work/base"

(cd "$work/base" && find . -type f | sed 's|^\./||') | sort >"$work/base.list"
git -C "$fork" ls-files --cached --others --exclude-standard . | sort -u >"$work/fork.list"

delta=()
while read -r path; do
  if [[ ! -f $fork/$path ]]; then
    delta+=("D $path")
  elif ! cmp -s "$work/base/$path" "$fork/$path"; then
    delta+=("M $path")
  fi
done <"$work/base.list"
while read -r path; do
  [[ -f $fork/$path ]] && delta+=("A $path")
done < <(comm -13 "$work/base.list" "$work/fork.list")

mapfile -t listed < <(sed -n 's/^| `\([^`]*\)` *|.*/\1/p' "$fork/upstream-patches.md")

status=0
used=()
for entry in "${delta[@]}"; do
  path=${entry#? }
  match=
  for pattern in "${listed[@]}"; do
    # shellcheck disable=SC2053
    if [[ $path == $pattern ]]; then match=$pattern; break; fi
  done
  if [[ -z $match ]]; then
    echo "unlisted fork change: $entry" >&2
    status=1
  else
    used+=("$match")
  fi
done
for pattern in "${listed[@]}"; do
  if [[ ! " ${used[*]} " == *" $pattern "* ]]; then
    echo "listed but identical to upstream: $pattern" >&2
    status=1
  fi
done

printf '%s\n' "${delta[@]}"
if ((status == 0)); then
  echo "fork delta matches upstream-patches.md (${#delta[@]} paths, base ${base:0:8})"
fi
exit $status
