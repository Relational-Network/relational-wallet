#!/usr/bin/env bash
set -euo pipefail

REQUIRED_YEAR="${REQUIRED_YEAR:-}"

file_list=$(git ls-files | grep -E '\.(rs|ts|tsx|js|jsx|py|go|java|kt|c|h|cpp|hpp|cs|swift|rb|php)$' || true)

if [[ -z "$file_list" ]]; then
  echo "No matching source files found."
  exit 0
fi

missing=0

while IFS= read -r file; do
  header=$(head -n 5 "$file")

  if ! grep -q "SPDX-License-Identifier: AGPL-3.0-or-later" <<<"$header"; then
    echo "Missing SPDX: $file"
    missing=1
  fi

  if [[ -n "$REQUIRED_YEAR" ]]; then
    if ! grep -q "Copyright (C) $REQUIRED_YEAR Relational Network" <<<"$header"; then
      echo "Missing Copyright ($REQUIRED_YEAR): $file"
      missing=1
    fi
  else
    if ! grep -Eq "Copyright \(C\) [0-9]{4} Relational Network" <<<"$header"; then
      echo "Missing Copyright (year): $file"
      missing=1
    fi
  fi

  if [[ $missing -eq 1 ]]; then
    continue
  fi

done <<<"$file_list"

if [[ $missing -eq 1 ]]; then
  exit 1
fi

echo "All checked files have SPDX and copyright headers."