<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/logo-horizontal-dark.svg">
    <img src="brand/logo-horizontal-light.svg" alt="CutZone" width="360">
  </picture>
</p>

<p align="center"><b>Create. Cut. Captivate.</b> · <b>أبدع. حرّر. أبهر.</b></p>

CutZone is a free, open-source video editor that runs on your machine: a native Rust
engine, a GPU compositor, local captions and background removal, and an automation
interface so scripts and AI agents can edit for you.

It is a **rebranded fork of [Concat](https://github.com/jub0t/Concat)** by Jareer Abdullah,
licensed AGPL-3.0-or-later. The engine, the project format and the crate names (`concat-*`)
are kept so upstream improvements still merge; what changes is the product around them.

> [!NOTE]
> **Status: early.** The name, icons, installers and updater are CutZone's. Arabic
> translation of the interface is complete, but the interface layout is not yet mirrored
> for right-to-left reading, and the Windows, Linux packaging, Android and iOS builds
> have been renamed but not yet built or run. See [Status](#status).

## Automation

Everything the window does goes through one API, which three front doors expose:

| Door | Use |
|---|---|
| `cutzone --mcp` | An MCP server over stdin/stdout, for AI agents. See [docs/MCP.md](docs/MCP.md). |
| JSON-RPC (and gRPC) | `concat-cli serve`, or Settings > Automation in the window, for scripts and services. |
| `concat-cli` | Probe, render and drive projects from a terminal. |
| `concat-cli autopilot` | Talking footage to a captioned vertical video with the silence cut. See [docs/AUTOPILOT.md](docs/AUTOPILOT.md). |

The API can read and write files as you can: turn it on only for programs you trust.

## Build from source

You need Rust 1.93.1 (pinned in `src/rust-toolchain.toml`), the **FFmpeg 7+** development
libraries, cmake and a C++ compiler. Ubuntu 24.04 ships FFmpeg 6.1, which is too old: use
a BtbN `shared` build and point `FFMPEG_DIR` at it (`scripts/ci/fetch.sh ffmpeg-linux64`
downloads the pinned one).

```sh
cd src
cargo run -p concat                    # the editor window; the program is named cutzone
cargo build --profile app -p concat    # the shipping binary
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
python3 scripts/locales.py --check     # after changing any interface text
scripts/check-brand.sh                 # from the repo root: the rebrand has not drifted
```

Building the window crate is memory-hungry because Slint generates one very large module:
**plan on 16 GB of RAM or more**; a 15 GB machine had its compiler killed during the
debug test build. Details: [CONTRIBUTING.md](CONTRIBUTING.md), [src/README.md](src/README.md).

## Status

| Area | State |
|---|---|
| Visible name, window, update channel, default folders | Done and tested |
| Icons, lockups with outlined wordmark, Android adaptive icon | Done; see [brand/](brand/) |
| Arabic interface text | Complete (1009 strings) |
| Autopilot: silence cut, 9:16 framing, Arabic captions that follow the cut | Works end to end on real speech and is checked by `scripts/e2e-autopilot.sh`; Arabic captions render correctly |
| Autopilot: transcription | Runs end to end with Whisper (checked in English, `tiny`). **Arabic has not been run** and its accuracy on dialects is unmeasured |
| Right-to-left layout mirroring | **Not started.** Slint 1.18 has no layout-direction support, so it is a staged refactor of about 220 horizontal layouts |
| Brand fonts (Space Grotesk, Inter, Cairo) inside the app | Not bundled |
| Windows, Linux, macOS, Android, iOS packaging | Renamed, **not yet built or run** |
| AI models | Still downloaded from the upstream project's release; to be mirrored |
| Identifier `com.cutzone.app` | **Placeholder** until a domain is registered; permanent once published |
| Trademark clearance for the name and mark | Not done |

## License and attribution

AGPL-3.0-or-later, see [LICENSE](LICENSE). Upstream copyright (Jareer Abdullah) and the
[third-party notices](THIRD_PARTY_NOTICES.md) are preserved. If you run a modified version
for others over a network, the AGPL requires you to offer them its source.

The CutZone name and logo are not licensed by the AGPL. "Concat" and its logo belong to
the upstream project; CutZone is described as a fork of it and is not endorsed by it.

---

<div dir="rtl">

## CutZone بالعربية

**CutZone** محرر فيديو مجاني ومفتوح المصدر يعمل على جهازك: محرك Rust أصلي، وتصيير على
المعالج الرسومي، وترجمة صوتية وإزالة خلفية محليتان، وواجهة أتمتة تتيح للبرامج ووكلاء
الذكاء الاصطناعي المونتاج نيابةً عنك.

هو نسخة معاد تسميتها من مشروع [Concat](https://github.com/jub0t/Concat) بترخيص
AGPL-3.0 أو أحدث، ونحتفظ بالمحرك وصيغة المشروع ليسهل دمج تحسينات المشروع الأصلي.

**الحالة: مبكرة.** ترجمة نصوص الواجهة العربية مكتملة، لكن **عكس التخطيط لقراءة اليمين لليسار
لم يبدأ بعد**، وحزم التثبيت لكل المنصات أُعيدت تسميتها ولم تُبنَ أو تُشغَّل بعد.

الأتمتة: الأمر `cutzone --mcp` لوكلاء الذكاء الاصطناعي، وخادم JSON-RPC عبر `concat-cli serve`
أو من الإعدادات ‹ الأتمتة، وأداة `concat-cli` لسطر الأوامر. يستطيع الخادم قراءة الملفات
وكتابتها بصلاحياتك، فلا تفعّله إلا لبرامج تثق بها.

</div>
