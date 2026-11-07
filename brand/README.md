# CutZone brand sources

CutZone is a rebranded fork of [Concat](https://github.com/jub0t/Concat), licensed
AGPL-3.0-or-later. Upstream copyright and licence notices stay in place. The `concat-*`
crate names are kept on purpose so upstream fixes still merge cleanly; only what a person
sees is renamed.

## What is here

| File | Use |
|---|---|
| `symbol.svg` | Full mark. **Dark backgrounds only**: its pale perforations and light gradient start wash out on white. |
| `symbol-simple.svg` | Mark without perforations, for 32 px and below. |
| `symbol-white.svg`, `symbol-black.svg` | One-colour marks on a transparent ground, for light backgrounds, print, embossing. The play triangle is knocked out with a mask, so there is no baked-in background. |
| `app-icon.svg`, `app-icon-simple.svg` | Rounded desktop icon (Windows, Linux, README). The simple one is used for 16 and 32 px. |
| `app-icon-fullbleed.svg` | Square, no rounding, no transparency: the **iOS and macOS source**. The platform applies its own mask. |
| `android-foreground.svg`, `android-monochrome.svg` | Adaptive icon layers on a 108 canvas. The mark is scaled to 55%, so its farthest point sits at radius 30.8, inside the 33 safe circle. Background colour is `#151320`. |
| `exports/` | Rendered PNGs (iOS 1024, Android layers, symbols). |

Regenerate everything with `node scripts/render-brand.cjs` (Playwright's Chromium and
ImageMagick). It also rewrites `assets/icons/*` and `concat.ico` **in place under their old
names**, so installers and CI keep working until the binary is renamed.

`scripts/check-brand.sh` fails if the Slint files drift from `concat-host/src/brand.rs`
or a locale value says "Concat" again.

## Decisions where the two guideline documents disagreed

| Topic | Chosen |
|---|---|
| Gradient | `#C4B5FD → #8B5CF6 → #6D28D9`, the one the artwork already uses |
| Logo clear space | 1 x the play-triangle height (the stricter rule) |
| Print minimum | 35 mm |
| Colour split in the editor | 75% neutral dark, 15% elevated and text, 10% violet |
| Reveal | Brand films use the 2.4 s ribbon storyboard. App launch uses the 650-900 ms splash and never delays loading. |

## Contrast rules (measured, WCAG 2.2)

- White text on `#7C3AED` buttons: 5.35, passes.
- `#A78BFA` text on the dark canvas: 7.24, passes. **Use it for links and text.**
- `#7C3AED` as text or a border on the canvas: 3.2-3.5. Fills and large shapes only.
- Border `#343044` on the canvas: 1.55. Fine for dividers, **not** for the outline of an input
  or a control, which needs 3:1. That token still has to be added.
- `#A78BFA` on white: 2.72, fails. On light grounds use Midnight text and Deep Purple.

## Lockups

`logo-horizontal-{dark,dark-bg,light,white,black}.svg` carry the wordmark as **outlined paths**
(no font needed to display them), shaped by HarfBuzz from Space Grotesk Bold with kerning and
-0.025em tracking (the supplied file used -0.045em, tighter than the guideline's -0.02 to -0.03).
Rebuild with `python3 scripts/build-wordmark.py` (needs `fonttools` and `uharfbuzz`; it downloads
the font, which is SIL OFL 1.1 and is not committed). `light` uses Midnight and Deep Purple, which
is 10:1 on white; `symbol.svg` and the dark lockups are for dark surfaces only.

## What the rename covers, and what it deliberately leaves

Renamed: the window, the executable (`cutzone`), the application identifier, the settings and
models folder, the Linux package (deb, rpm, pacman, Nix), the Windows installers (with **new
GUIDs**, so they never upgrade upstream's), the macOS and iOS bundles, the Android package and
launcher icon, the MCP server's name, `--version` and `--help`, and the updater's repository.

Kept on purpose, because changing them would break compatibility or upstream merges:
the `concat-*` crate names and `-p concat`; the project format (`concat.json`, the `"concat"` key
in project files, so projects open in both products); the API contract key; the effect author
id; the `CONCAT_VERSION`/`CONCAT_STAGE` variables shared between CI steps; `concat-cli`
(a developer tool the installers do not ship) and `concat-mcp`; the installer definition and
icon file names (`concat.wxs`, `concat.iss`, `concat.ico`, `concat_logo_*.png`).

`dirs.rs` keeps its "adopt a former folder" code but points it at a name nothing creates:
upstream's former identifier would otherwise have moved another product's settings and models
into CutZone's folder.

## Not done yet

- **Right-to-left layout.** Slint 1.18 has no layout-direction property (only
  `FlexboxLayout`'s `row-reverse`), so mirroring is a refactor of ~220 `HorizontalLayout`s and
  ~540 absolute `x:` positions across 75 files. The timeline, ruler and transport must stay
  left-to-right (time does not reverse). It needs a way to see the window, which was not available.
- Space Grotesk, Inter and Cairo are not bundled in the app; it keeps its existing fonts, with
  Noto Sans Arabic UI as the Arabic fallback. Switching needs the same visual check.
- **Nothing outside Linux x86-64 has been built or run**: the Windows, macOS, Android and iOS
  packaging was renamed by reading it. The window crate type-checks, but its tests were not run
  here (a 15 GB machine kills the compiler building them).
- The application identifier `com.cutzone.app` is a placeholder; it is permanent once published.
- The translated READMEs (`docs/README.*.md`) and `scripts/readme/` still describe Concat, and
  `gen.py` rewrites the English README's language line: do not run it until it is ported.
- AI models are still downloaded from upstream's `models-v1` release.
- Croatian strings keep their original grammar around the name (`CutZonea`, `CutZoneu`) and
  need a native reader.
- No `TRADEMARK.md` for CutZone, and no trademark clearance for the name or the mark.
- `.invalid` placeholder maintainer address in `assets/linux/nfpm.yaml`.
