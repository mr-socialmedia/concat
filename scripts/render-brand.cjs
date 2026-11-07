// Renders brand/*.svg into the PNG and ICO files the app, the installers and
// the READMEs read. Run from anywhere: node scripts/render-brand.cjs
// Needs Playwright's Chromium and ImageMagick (convert) on PATH.
//
// 16 and 32 px come from the simplified mark: the film perforations are two
// pixels tall at 24 px and turn into noise (brand guidelines, 2.3 and 2.5).
const { chromium } = require('playwright');
const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const read = (name) => fs.readFileSync(path.join(root, 'brand', name), 'utf8');
const at = (...p) => path.join(root, ...p);

const jobs = [];
const icon = (size, file) => jobs.push({ svg: size <= 32 ? 'app-icon-simple.svg' : 'app-icon.svg', size, out: file });
for (const s of [512, 256, 128, 64, 32, 16]) icon(s, at('assets/icons', `concat_logo_${s}.png`));
icon(48, at('brand/exports/.ico-48.png'));
jobs.push({ svg: 'app-icon.svg', size: 512, out: at('assets/concat_logo_512.png') });
jobs.push({ svg: 'symbol.svg', size: 512, out: at('assets/concat-mark-dark.png') });
jobs.push({ svg: 'symbol.svg', size: 512, out: at('brand/exports/symbol-512.png') });
jobs.push({ svg: 'symbol-white.svg', size: 512, out: at('brand/exports/symbol-white-512.png') });
jobs.push({ svg: 'symbol-black.svg', size: 512, out: at('brand/exports/symbol-black-512.png') });
jobs.push({ svg: 'app-icon-fullbleed.svg', size: 1024, out: at('brand/exports/ios-appicon-1024.png'), opaque: true });
jobs.push({ svg: 'app-icon-fullbleed.svg', size: 512, out: at('brand/exports/macos-source-512.png'), opaque: true });
jobs.push({ svg: 'android-foreground.svg', size: 432, out: at('brand/exports/android-foreground-432.png') });
// The shipping Android app's launcher icons (src/crates/concat-android/res). minSdk is 26, so
// every phone uses the adaptive layers; the plain icon is the fallback for launchers that ignore them.
const res = (...p) => at('src/crates/concat-android/res', ...p);
const dpi = { mdpi: [48, 108], hdpi: [72, 162], xhdpi: [96, 216], xxhdpi: [144, 324], xxxhdpi: [192, 432] };
for (const [d, [legacy, layer]] of Object.entries(dpi)) {
  jobs.push({ svg: legacy <= 48 ? 'app-icon-simple.svg' : 'app-icon.svg', size: legacy, out: res(`mipmap-${d}`, 'ic_launcher.png') });
  jobs.push({ svg: 'android-foreground.svg', size: layer, out: res(`mipmap-${d}`, 'ic_launcher_foreground.png') });
  jobs.push({ svg: 'android-monochrome.svg', size: layer, out: res(`mipmap-${d}`, 'ic_launcher_monochrome.png') });
}
jobs.push({ svg: 'android-monochrome.svg', size: 432, out: at('brand/exports/android-monochrome-432.png') });

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  for (const j of jobs) {
    const svg = read(j.svg).replace(/<svg ([^>]*?)width="\d+" height="\d+"/, `<svg $1width="${j.size}" height="${j.size}"`);
    await page.setViewportSize({ width: j.size, height: j.size });
    await page.setContent(`<!doctype html><style>html,body{margin:0;background:transparent}svg{display:block}</style>${svg}`);
    fs.mkdirSync(path.dirname(j.out), { recursive: true });
    await page.screenshot({ path: j.out, omitBackground: true, clip: { x: 0, y: 0, width: j.size, height: j.size } });
    if (j.opaque) execFileSync('convert', [j.out, '-background', '#151320', '-alpha', 'remove', '-alpha', 'off', j.out]);
    console.log('wrote', path.relative(root, j.out));
  }
  await browser.close();
  const ico = at('assets/icons/concat.ico');
  const parts = ['16', '32'].map((s) => at('assets/icons', `concat_logo_${s}.png`)).concat(at('brand/exports/.ico-48.png'),
    ['64', '128', '256'].map((s) => at('assets/icons', `concat_logo_${s}.png`)));
  execFileSync('convert', [...parts, ico]);
  fs.unlinkSync(at('brand/exports/.ico-48.png'));
  console.log('wrote', path.relative(root, ico));
})();
