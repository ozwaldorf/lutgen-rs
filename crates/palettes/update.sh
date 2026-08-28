#!/usr/bin/env bash
# Fetch every source file referenced by palettes.toml and check that the
# committed colors are all present upstream.
#
# Usage:
#   ./update.sh                 check every palette
#   ./update.sh NAME [NAME...]  check only the named palettes
#
# Sources are listed as GitHub /blob/ URLs; they are rewritten to
# raw.githubusercontent.com to fetch. Responses are cached under
# .cache/ so re-runs are cheap; delete it to force a refetch.
#
# A palette passes when every committed color appears somewhere in at least
# one of its source files. Upstream files usually define more colors than we
# keep, so this is a subset check, not an equality check.

set -uo pipefail

cd "$(dirname "$0")"

TOML=palettes.toml
CACHE=.cache
# Concurrent downloads while warming the cache.
JOBS=${JOBS:-16}

mkdir -p "$CACHE"

pass=0
fail=0
nosrc=0
unreachable_n=0
drift=0
failed_names=()
nosrc_names=()
unreachable_names=()
drift_names=()

# Pull the hex colors out of a source file.
#
# Handles the two shapes every upstream format reduces to:
#   - literal hex, with or without a leading '#' (yaml, json, lua, vim, css,
#     toml, xrdb, ...). Bare hex is only taken from quoted base16 fields to
#     avoid matching arbitrary 6-char words.
#   - .itermcolors plists, which store float RGB components instead of hex.
#     Keys inside each color dict are alphabetized (Alpha, Blue, Color Space,
#     Green, Red), so components are buffered by name rather than by order.
extract_colors() {
  local file=$1
  if grep -q '<key>Red Component</key>' "$file"; then
    awk '
      /<key>(Red|Green|Blue) Component<\/key>/ {
        ch = $0; gsub(/.*<key>| Component<\/key>.*/, "", ch)
        getline line
        v = line; gsub(/.*<real>|<\/real>.*/, "", v)
        c[ch] = v + 0
        if ("Red" in c && "Green" in c && "Blue" in c) {
          printf "#%02x%02x%02x\n", int(c["Red"]*255+0.5), int(c["Green"]*255+0.5), int(c["Blue"]*255+0.5)
          delete c
        }
      }
    ' "$file"
  else
    # Take the first 6 digits of each hex literal; sources written as #RRGGBBAA
    # carry the same color with an alpha suffix.
    grep -oiE '#[0-9a-f]{6}([0-9a-f]{2})?' "$file" | cut -c1-7
    # C sources spell colors as RGB_(0xNN, 0xNN, 0xNN).
    grep -oiE 'RGB_\(0x[0-9a-f]{2}, ?0x[0-9a-f]{2}, ?0x[0-9a-f]{2}\)' "$file" |
      sed -E 's/RGB_\(0x([0-9a-fA-F]{2}), ?0x([0-9a-fA-F]{2}), ?0x([0-9a-fA-F]{2})\)/#\1\2\3/'
    # base16 yaml writes bare hex: base00: "031A16"
    grep -oiE '"[0-9a-f]{6}"|'\''[0-9a-f]{6}'\''' "$file" | tr -d "\"'" | sed 's/^/#/'
    # CSS shorthand: #abc is #aabbcc.
    grep -oiE '#[0-9a-f]{3}([^0-9a-f]|$)' "$file" | cut -c1-4 |
      sed -E 's/#(.)(.)(.)/#\1\1\2\2\3\3/'
  fi | tr 'A-F' 'a-f' | sort -u
}

# Read one array field (src or palette) out of a palette's TOML block.
read_field() {
  local name=$1 field=$2
  awk -v name="[$name]" -v field="$field = [" '
    $0 == name { inblock = 1; next }
    inblock && /^\[/ { exit }
    inblock && index($0, field) == 1 {
      # Inline empty array, e.g. src = []
      if ($0 ~ /\[\]$/) exit
      inarr = 1; next
    }
    inarr && /^\]/ { exit }
    inarr { gsub(/^[ \t]*"|",?[ \t]*$/, ""); if ($0 != "") print }
  ' "$TOML"
}

# Read a scalar field out of a palette's TOML block.
read_flag() {
  local name=$1 field=$2
  awk -v name="[$name]" -v field="$field = " '
    $0 == name { inblock = 1; next }
    inblock && /^\[/ { exit }
    inblock && index($0, field) == 1 { sub(field, ""); print; exit }
  ' "$TOML"
}

fetch() {
  local url=$1
  local raw=${url/github.com/raw.githubusercontent.com}
  raw=${raw/\/blob\//\/}
  local key
  key=$(printf '%s' "$url" | cksum | tr -d ' ')
  local out="$CACHE/$key"
  if [[ ! -s $out ]]; then
    # Retry: a dropped connection mid-run would otherwise be reported as a
    # palette missing every color.
    curl -sfL --retry 3 --retry-delay 1 --connect-timeout 15 --max-time 60 \
      "$raw" -o "$out" || { rm -f "$out"; return 1; }
  fi
  printf '%s' "$out"
}

# Which palettes to check.
if [[ $# -gt 0 ]]; then
  names=("$@")
else
  mapfile -t names < <(grep -oP '^\[\K[^]]+' "$TOML")
fi

# Warm the cache first. Fetching is the slow part and each URL is independent,
# so download them concurrently; the check below then reads from disk. Many
# palettes share a source, hence the dedupe.
export CACHE
export -f fetch
for name in "${names[@]}"; do
  read_field "$name" src
done | sort -u | xargs -r -P "$JOBS" -I {} bash -c 'fetch "$@" >/dev/null 2>&1' _ {}

for name in "${names[@]}"; do
  mapfile -t srcs < <(read_field "$name" src)

  if [[ ${#srcs[@]} -eq 0 ]]; then
    printf 'NO SRC  %s\n' "$name"
    nosrc=$((nosrc + 1))
    nosrc_names+=("$name")
    continue
  fi

  # Union of every color found across this palette's sources.
  unreachable=""
  colors="$CACHE/.colors.$$"
  : > "$colors"
  for url in "${srcs[@]}"; do
    if file=$(fetch "$url"); then
      extract_colors "$file" >> "$colors"
    else
      unreachable=$url
    fi
  done
  upstream=$(sort -u "$colors")
  rm -f "$colors"

  # An unreachable source is a network problem, not a palette problem; saying
  # "missing colors" here would point at the wrong culprit.
  if [[ -n $unreachable ]]; then
    printf 'FETCH   %s: could not fetch %s\n' "$name" "$unreachable"
    unreachable_n=$((unreachable_n + 1))
    unreachable_names+=("$name")
    continue
  fi

  committed=$(read_field "$name" palette | tr 'A-F' 'a-f' | sort -u)
  missing=$(comm -23 <(printf '%s\n' "$committed") <(printf '%s\n' "$upstream"))

  n_missing=$(printf '%s\n' "$missing" | grep -c .)
  n_total=$(printf '%s\n' "$committed" | grep -c .)

  if [[ -z $missing ]]; then
    printf 'OK      %s (%d src)\n' "$name" "${#srcs[@]}"
    pass=$((pass + 1))
  elif [[ $(read_flag "$name" incomplete_src) == true ]]; then
    # The palette declares colors its sources do not carry, hand-verified
    # once and not re-checkable from any file.
    printf 'DRIFT   %s (%d/%d): %s\n' "$name" "$n_missing" "$n_total" \
      "$(printf '%s' "$missing" | tr '\n' ' ')"
    drift=$((drift + 1))
    drift_names+=("$name")
  else
    # A palette missing every color points at the wrong file entirely (or one
    # that only stores color references, resolved by the theme's build step).
    # A handful missing usually means upstream drifted, or the palette was
    # drawn from a fuller source than the one linked.
    if [[ $n_missing -eq $n_total ]]; then
      printf 'WRONG   %s: no color matched (%d)\n' "$name" "$n_total"
    else
      printf 'MISSING %s (%d/%d): %s\n' "$name" "$n_missing" "$n_total" \
        "$(printf '%s' "$missing" | tr '\n' ' ')"
    fi
    fail=$((fail + 1))
    failed_names+=("$name")
  fi
done

printf '\n%d ok, %d drift, %d missing colors, %d without sources, %d unreachable\n' \
  "$pass" "$drift" "$fail" "$nosrc" "$unreachable_n"

if [[ ${#failed_names[@]} -gt 0 ]]; then
  printf 'missing colors: %s\n' "${failed_names[*]}"
fi
if [[ ${#nosrc_names[@]} -gt 0 ]]; then
  printf 'without sources: %s\n' "${nosrc_names[*]}"
fi
if [[ ${#unreachable_names[@]} -gt 0 ]]; then
  printf 'unreachable: %s\n' "${unreachable_names[*]}"
fi
if [[ ${#drift_names[@]} -gt 0 ]]; then
  printf 'drift (incomplete_src, hand-verified): %s\n' "${drift_names[*]}"
fi

[[ $fail -eq 0 && $nosrc -eq 0 && $unreachable_n -eq 0 ]]
