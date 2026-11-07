#!/usr/bin/env bash
# Fails when the product name the Slint files repeat drifts from
# src/crates/concat-host/src/brand.rs, which Slint cannot read.
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
name=$(sed -n 's/^pub const NAME: &str = "\(.*\)";/\1/p' "$root/src/crates/concat-host/src/brand.rs")
[ -n "$name" ] || { echo "check-brand: NAME not found in brand.rs" >&2; exit 2; }
ui="$root/src/crates/concat/ui"
status=0
grep -q "title: \"$name\";" "$ui/app.slint" || { echo "check-brand: app.slint title is not $name" >&2; status=1; }
grep -q "text: \"$name \" + root.version;" "$ui/start.slint" || { echo "check-brand: start.slint version line is not $name" >&2; status=1; }
# The old name must not come back into a locale value.
if grep -h '^\s*"[^"]*":\s*".*Concat' "$root"/src/crates/concat/locales/*.json | grep -q .; then
  echo "check-brand: a locale value still says Concat" >&2; status=1
fi
[ "$status" -eq 0 ] && echo "check-brand: ok ($name)"
exit "$status"
