#!/usr/bin/env bash
# Guards the rebrand against drifting back toward the upstream product.
# Each check names a failure that would reach a user: another product's updates,
# its data folder, its installer's upgrade slot, or its package name.
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
status=0
fail() { echo "check-brand: $*" >&2; status=1; }

name=$(sed -n 's/^pub const NAME: &str = "\(.*\)";/\1/p' src/crates/concat-host/src/brand.rs)
ident=$(sed -n 's/^pub const IDENTIFIER: &str = "\(.*\)";/\1/p' src/crates/concat-host/src/brand.rs)
pkg=$(sed -n 's/^pub const PACKAGE: &str = "\(.*\)";/\1/p' src/crates/concat-host/src/brand.rs)
[ -n "$name" ] && [ -n "$ident" ] && [ -n "$pkg" ] || { echo "check-brand: brand.rs constants not found" >&2; exit 2; }

# 1. Slint cannot read brand.rs, so what it repeats must match.
ui=src/crates/concat/ui
grep -q "title: \"$name\";" $ui/app.slint || fail "app.slint title is not $name"
grep -q "text: \"$name \" + root.version;" $ui/start.slint || fail "start.slint version line is not $name"

# 2. No locale value says the old name.
grep -h '^\s*"[^"]*":\s*".*Concat' src/crates/concat/locales/*.json | grep -q . && fail "a locale value still says Concat"

# 3. The executable is named for this product.
grep -A1 '^\[\[bin\]\]' src/crates/concat/Cargo.toml | grep -q "name = \"$pkg\"" || fail "the window's binary is not $pkg"

# 4. The identifier and package name are the product's own, everywhere that builds or ships.
old=$(grep -rIln 'app\.concat\.editor\|org\.standalones' \
  --exclude-dir=target --exclude-dir=.git --exclude-dir=docs --exclude-dir=readme \
  --exclude=dirs.rs --exclude=brand.rs --exclude=README.md --exclude=TRANSLATING.md --exclude=check-brand.sh . || true)
[ -z "$old" ] || fail "the upstream identifier is still in: $old"
grep -rIn "$ident" android-native src/crates/concat-android src/scripts/ios-app.sh .github/workflows/build-app.yml >/dev/null || fail "$ident is not in the platform files"

# 5. Windows treats a GUID as the product's identity: sharing upstream's would make one installer upgrade the other.
grep -rIn '6C4D5D8E-9B2A-4F3B\|2F0B7C51-6E4D-4A1B\|7B1E5C3A-3B9E-4F0B' assets/windows >/dev/null && fail "an installer still uses upstream's GUID"

# 6. Linux packages must not share a name with upstream's.
grep -q "^pkgname=$pkg$" assets/arch/PKGBUILD || fail "PKGBUILD pkgname is not $pkg"
grep -q "^name: $pkg$" assets/linux/nfpm.yaml || fail "nfpm.yaml name is not $pkg"

# 7. The updater and the links point at this repository.
# Comments that cite upstream issues are history, and the guard's own test names the owner on purpose.
live=$(grep -rIn 'jub0t' src/crates/concat-host/src/brand.rs src/crates/concat-host/src/updates.rs src/crates/concat/src/panes/start.rs \
  assets/windows flake.nix assets/linux assets/arch | grep -vE ':[0-9]+:\s*(//|#|;)' | grep -v 'contains("jub0t")' || true)
[ -z "$live" ] || fail "a link or the updater points at upstream: $live"

# 8. Outlined lockups carry no live text, which would fall back to a system font.
grep -l '<text\|font-family' brand/logo-horizontal-*.svg 2>/dev/null | grep -q . && fail "a lockup has live text"

[ "$status" -eq 0 ] && echo "check-brand: ok ($name, $ident, $pkg)"
exit "$status"
