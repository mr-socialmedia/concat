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

## Not done yet

- Wordmark is live text in the supplied files; it must be converted to outlines with Space
  Grotesk Bold (SIL OFL) before it ships. No light-background horizontal logo exists yet.
- Space Grotesk, Inter and Cairo are not bundled in the app; the editor still uses its
  existing fonts, with Noto Sans Arabic UI as the Arabic fallback.
- Application identifier (`app.concat.editor`), the binary name and the installers.
- Croatian strings keep the original grammar around the name (`CutZonea`, `CutZoneu`)
  and need a native reader.
- Trademark clearance for "CutZone" and the mark has not been done.
