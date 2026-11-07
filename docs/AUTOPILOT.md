# CutZone autopilot

Talking footage in, a vertical video out: the silence cut, Arabic captions laid over the words,
framed 9:16. It is `concat-cli autopilot`, built on the engine's own API, so everything it does is
something a person could do in the window, and the project it leaves behind opens in the editor.

```sh
concat-cli autopilot talk.mp4 --plan-only                      # what would be cut; changes nothing
concat-cli autopilot talk.mp4 --out reel.mp4                   # cut and frame
concat-cli autopilot talk.mp4 --out reel.mp4 --segments words.json
concat-cli autopilot talk.mp4 --out reel.mp4 --transcribe --language ar --model small
```

| Option | |
|---|---|
| `--plan-only` | Prints the speech, the stretches kept and the pauses cut, as JSON. |
| `--spans FILE` | `[[start, end], ...]` in seconds: use these instead of listening. |
| `--segments FILE` | `[{"start":0,"end":2,"text":"..."}]` in the recording's own time. They are moved to follow the cut. |
| `--transcribe`, `--language ar`, `--model small` | Hear the words with Whisper. `small` for Arabic; `tiny` and `base` are rough on it. |
| `--keep-pauses 0.45` | A pause shorter than this many seconds stays. |
| `--no-cut`, `--landscape` | Keep every pause; keep 16:9. |
| `--font FILE` (repeat), `--font-family Cairo` | Caption faces. Give a **static** face per weight. |

## What happens

1. **Listen.** Silero VAD, the detector the transcriber already runs first, finds the speech.
2. **Plan.** Speech gets 0.10 s before and 0.20 s after; pauses under 0.45 s stay (cutting them
   sounds rushed); bursts under 0.20 s are clicks, not speech; no speech at all is refused rather than
   turned into an empty video. (`concat-autopilot/src/cuts.rs`)
3. **Edit.** A project is created at 1080 x 1920 and 30 fps, the recording is placed and covers the
   frame, and each pause is split off and rippled away, **last one first**, so the times of those not
   yet cut do not move. A pause is left in, and reported, if the piece about to be deleted is not as
   long as the pause: cutting it could remove more than intended.
4. **Caption.** The words are moved through the cut (a line spoken across a removed pause becomes one
   line; a line entirely in the cut disappears), broken into pages of at most two lines of 22 letters
   (Arabic vowel marks and shadda add no width, so they are not counted), and held long enough to read
   without running into the next one. (`captions.rs`)
5. **Export** and measure: the report states the output's size, length and whether it has sound.

## Measured here

On a 24.5 s recording made from the repository's own human voice samples, with speech at
0-6, 9-15 and 17.5-22.5 s:

- Silero heard 0.10-6.08, 8.99-15.04 and 17.51-22.56: within 0.1 s at every edge. It took 181 ms.
- 4 pauses cut, none skipped; 24.5 s became 17.4 s (expected 17.41), 1080 x 1920, sound kept, audio
  and video the same length.
- A second pass over the output finds 0.02 s left to cut, and the pauses that remain are about 0.3 s.
- Arabic captions land on the right frames after the cut, with joined letters, tanwin and shadda,
  mixed Latin words and digits in the right place, and punctuation on the correct side of a
  right-to-left line.

`scripts/e2e-autopilot.sh` repeats this and fails if any of it stops being true.

## Things learned the hard way

- **Sizes and offsets are fractions of the frame.** A font size of 96 is 96 frames tall. Padding is a
  fraction too: a caption "background" of 0.5 covers half the frame, and several of them dim the
  earlier captions. The autopilot uses no background box.
- **A variable font's weight axis is ignored.** Cairo downloaded from Google Fonts is variable, and
  weight 700 drew as regular. Register static Regular and Bold faces under one family and the engine
  picks by weight. (Space Grotesk's lockups are outlined, so they are unaffected.)
- **Without a graphics card, name the software driver.** On a machine with no GPU (a container, a
  build server), the renderer waited over four minutes on a driver that never answered.
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json WGPU_BACKEND=vulkan` made it instant. A cloud
  worker needs either a GPU or this.
- **Edits stay in memory until saved**, so a project reopened in a new process is empty. The autopilot
  works in one process and saves before exporting.

## Not done, or not known

- **Arabic transcription accuracy is unmeasured.** No Arabic speech was available here, so only
  English was exercised for the listening step, and only through the detector. Whisper's Arabic is
  weakest on dialects (Gulf, Egyptian, Levantine, Maghrebi) and on the smaller models; measure on your
  own recordings before relying on it.
- **`--transcribe` has not completed a run here.** Whisper's models are fetched from huggingface.co,
  which this machine's network does not allow; the project's own mirror answered 404 for
  `ggml-tiny.bin`, the one file tried. The code path compiles and fails cleanly with that message.
- Captions use the font family you give; the brand fonts are not bundled yet.
- The debug build took 45 s for 17 s of 1080 x 1920 on a software renderer. A release build or a
  GPU will be much faster; it has not been measured.

---

<div dir="rtl">

## الأتمتة بالعربية

`concat-cli autopilot` يحوّل تسجيل كلام إلى فيديو عمودي 9:16: يقص الصمت، ويضع كابشنز عربية
على الكلمات، ويؤطّر الصورة. يعمل عبر الـAPI نفسه الذي تستخدمه النافذة، فالمشروع الناتج يُفتح في
المحرر.

**ما تحقق بالقياس:** كاشف الكلام أصاب حدود المقاطع بفارق أقل من 0.1 ثانية، وقُصّت 4 وقفات من 4
(24.5 ث صارت 17.4 ث)، وبقي الصوت متزامنًا، وسقطت الكابشنز العربية على الإطارات الصحيحة بعد القص
بحروف متصلة وتنوين وشدّة وترتيب صحيح للكلمات اللاتينية داخل الجملة.

**ما لم يتحقق:** دقة التفريغ العربي نفسه. لم يتوفر صوت عربي للاختبار، ونماذج Whisper تُجلب من
huggingface.co المحجوب في بيئة التطوير هذه. قِس الدقة على تسجيلاتك قبل الاعتماد عليها، خصوصًا
اللهجات، واستخدم النموذج `small` على الأقل.

</div>
