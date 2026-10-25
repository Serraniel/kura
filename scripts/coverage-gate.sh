#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Fails when a crate's line coverage is below the minimum.
# Usage: coverage-gate.sh <llvm-cov json> <minimum percent> <crate>...
set -euo pipefail

report=$1 min=$2
shift 2
status=0
for crate in "$@"; do
  read -r covered count < <(jq -r --arg dir "crates/$crate/" '
    [.data[0].files[] | select(.filename | contains($dir)) | .summary.lines]
    | "\(map(.covered) | add // 0) \(map(.count) | add // 0)"' "$report")
  if [ "$count" -eq 0 ]; then
    echo "::error::$crate: no coverage data"
    status=1
    continue
  fi
  pct=$(( covered * 1000 / count ))
  printf '%s: %d.%d %% (%d of %d lines)\n' "$crate" $((pct / 10)) $((pct % 10)) "$covered" "$count"
  if [ $((pct / 10)) -lt "$min" ]; then
    echo "::error::$crate line coverage is below $min %"
    status=1
  fi
done
exit $status
