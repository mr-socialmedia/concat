#!/usr/bin/env bash
# End-to-end check of `concat-cli autopilot` on real human speech.
#
# Builds a 24.5 s recording from the repository's own voice samples with a known layout (speech
# 0-6, silence 3 s, speech 6 s, silence 2.5 s, speech 5 s, silence 2 s), turns it into a vertical
# captioned video, and checks the result: size, length, sound, every pause cut, and that a second
# pass over the output finds nothing left to cut.
#
#   scripts/e2e-autopilot.sh [path/to/concat-cli]
#
# Needs ffmpeg and python3 on PATH and a built concat-cli (cd src && cargo build -p concat-cli).
# Captions use whatever Arabic-capable font the system has; set CAIRO_REGULAR and CAIRO_BOLD to
# static Cairo faces to see the brand font.
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cli=${1:-$root/src/target/debug/concat-cli}
[ -x "$cli" ] || { [ -x "$cli.exe" ] && cli=$cli.exe; }   # Windows builds are concat-cli.exe
[ -x "$cli" ] || { echo "no concat-cli at $cli: build it first" >&2; exit 2; }
# Windows has python and the py launcher, not python3.
py=python3; command -v python3 >/dev/null || py=python; command -v "$py" >/dev/null || py="py -3"
command -v ffmpeg >/dev/null || { echo "ffmpeg is needed to make the test recording" >&2; exit 2; }

voices=$root/src/crates/concat-speech/voices
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"

# No graphics card (a build machine, a container): ask for the software Vulkan driver by name.
# Left to choose, the renderer can wait on a driver that never answers.
if [ ! -d /dev/dri ] && [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan
fi
# The detector's model is written beside the settings: keep the test's out of the real ones.
export XDG_DATA_HOME=$work/data XDG_CONFIG_HOME=$work/config
if command -v cygpath >/dev/null; then   # Git Bash: Windows keeps them under these, in Windows paths
  export APPDATA=$(cygpath -w "$work/config") LOCALAPPDATA=$(cygpath -w "$work/data")
fi

# The system's ffmpeg must use the system's libraries. LD_LIBRARY_PATH points at the FFmpeg that
# concat-cli links: LGPL, so without libx264, but with the same library names as a system FFmpeg of
# the same major version, which would load them and fail with "Unknown encoder 'libx264'".
ffmpeg() { LD_LIBRARY_PATH= command ffmpeg "$@"; }
venc=(-c:v libx264 -preset veryfast)
ffmpeg -hide_banner -encoders 2>/dev/null | grep -q libx264 || venc=(-c:v mpeg4 -q:v 3)

ffmpeg -v error -y -t 6 -i "$voices/mark-f-smith.flac" -ar 16000 -ac 1 a.wav
ffmpeg -v error -y -t 6 -i "$voices/elizabeth-klett.flac" -ar 16000 -ac 1 b.wav
ffmpeg -v error -y -t 5 -i "$voices/bob-neufeld.flac" -ar 16000 -ac 1 c.wav
for s in 3 2.5 2; do ffmpeg -v error -y -f lavfi -i anullsrc=r=16000:cl=mono -t $s "s$s.wav"; done
printf "file 'a.wav'\nfile 's3.wav'\nfile 'b.wav'\nfile 's2.5.wav'\nfile 'c.wav'\nfile 's2.wav'\n" > list.txt
ffmpeg -v error -y -f concat -safe 0 -i list.txt -c copy talk.wav
ffmpeg -v error -y -f lavfi -i testsrc2=size=1280x720:rate=30 -i talk.wav -t 24.5 \
  "${venc[@]}" -pix_fmt yuv420p -c:a aac -shortest talk.mp4

cat > segments.json <<'J'
[{"start":0.6,"end":5.9,"text":"مرحبًا بكم في CutZone، اليوم سنتعلم كيف نحرّر الفيديو بسرعة"},
 {"start":9.0,"end":14.8,"text":"أبدع. حرّر. أبهر. هذه قصتنا مع المونتاج التلقائي باللغة العربية"},
 {"start":17.6,"end":22.4,"text":"جرّب الآن مجانًا على جهازك بدون رفع ملفاتك"}]
J
fonts=()
[ -n "${CAIRO_REGULAR:-}" ] && fonts+=(--font "$CAIRO_REGULAR")
[ -n "${CAIRO_BOLD:-}" ] && fonts+=(--font "$CAIRO_BOLD")

"$cli" autopilot talk.mp4 --out out.mp4 --segments segments.json --work-dir proj "${fonts[@]}" > report.json 2> stderr.txt
"$cli" autopilot out.mp4 --plan-only > again.json 2>/dev/null

$py - <<'PY'
import json, sys
r = json.load(open("report.json")); again = json.load(open("again.json"))
out = r["output"]; failures = []
def check(ok, what):
    print(("PASS  " if ok else "FAIL  ") + what)
    if not ok: failures.append(what)
check((out["width"], out["height"]) == (1080, 1920), f"vertical 1080x1920 (got {out['width']}x{out['height']})")
check(out["hasAudio"], "the sound is kept")
check(abs(out["duration"] - r["keptSeconds"]) < 0.15, f"length {out['duration']} s matches the {r['keptSeconds']} s kept")
check(r["cutsApplied"] == len(r["cuts"]) and not r["cutsSkipped"], f"every pause cut ({r['cutsApplied']} of {len(r['cuts'])}, none skipped)")
check(r["removedSeconds"] > 5, f"{r['removedSeconds']} s removed ({r['removedPercent']}%)")
check(r["captions"] >= 3, f"{r['captions']} captions placed")
check(again["removedSeconds"] < 0.1, f"a second pass finds nothing left to cut ({again['removedSeconds']} s)")
sys.exit(1 if failures else 0)
PY
