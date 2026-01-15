#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <from_year> <to_year>"
  exit 1
fi

from_year="$1"
to_year="$2"

if [[ ! "$from_year" =~ ^[0-9]{4}$ || ! "$to_year" =~ ^[0-9]{4}$ ]]; then
  echo "Years must be four digits."
  exit 1
fi

files=$(git ls-files | grep -E '\.(rs|ts|tsx|js|jsx|py|go|java|kt|c|h|cpp|hpp|cs|swift|rb|php)$' || true)

if [[ -z "$files" ]]; then
  echo "No matching source files found."
  exit 0
fi

updated=0

while IFS= read -r file; do
  header=$(head -n 5 "$file")

  if grep -q "Copyright (C) $from_year Relational Network" <<<"$header"; then
    sed -i "s/Copyright (C) $from_year Relational Network/Copyright (C) $to_year Relational Network/g" "$file"
    echo "Updated: $file"
    updated=$((updated + 1))
  fi

done <<<"$files"

echo "Updated $updated file(s)."