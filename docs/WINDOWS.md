# Trying CutZone on Windows

**Nothing here has been built or run on Windows.** Everything in this repository was verified on
Linux x86-64. The steps mirror the Windows job in `.github/workflows/ci.yml`, which does build there,
but `scripts/windows-dev-env.ps1` itself has not been run. Report what breaks.

## Once

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --passive"
winget install Rustlang.Rustup Kitware.CMake Git.Git
```

Open a **new** PowerShell window afterwards so the tools are on the path.

## Every time you open a window

```powershell
git clone https://github.com/mr-socialmedia/concat
cd concat
git checkout claude/laughing-lovelace-lt1w8w
. .\scripts\windows-dev-env.ps1      # the leading dot matters
cd src
cargo build -p concat-cli            # the first build takes a long time
```

The script downloads LLVM, FFmpeg and ONNX Runtime into `%USERPROFILE%\cutzone-deps` the first time,
checking each against its pinned digest, and sets variables for that window only.

## Try the autopilot

```powershell
$cli = ".\target\debug\concat-cli.exe"
& $cli autopilot C:\videos\talk.mp4 --plan-only
& $cli autopilot C:\videos\talk.mp4 --out C:\videos\reel.mp4 *> $null
```

Notes for PowerShell:

- The report is JSON on standard output and Arabic text in it may look garbled in the console. Send
  it to a file (`> report.json`) and open it in an editor, or run `chcp 65001` first.
- A words file written by PowerShell or Notepad starts with a byte-order mark. CutZone accepts that.
- `--transcribe --language ar --model small` downloads the model (about 466 MB) the first time. Your
  network must reach `huggingface.co`.
- Captions need an Arabic-capable font. Windows has several (Segoe UI, Tahoma); for the brand look
  pass static Cairo faces with `--font`, see `docs/AUTOPILOT.md`.

## The self-checking test

It is a bash script, so run it from Git Bash (not PowerShell). It needs `ffmpeg` on the path, which
`windows-dev-env.ps1` provides for that window:

```powershell
& "$env:ProgramFiles\Git\bin\bash.exe" scripts/e2e-autopilot.sh src/target/debug/concat-cli.exe
```

Seven `PASS` lines is success. This script has also not run on Windows.

## The window

```powershell
cargo run -p concat                  # the program is named cutzone.exe
```

Building the window needs a lot of memory: plan on 16 GB. CI's own Windows job sets
`CARGO_PROFILE_DEV_DEBUG=0` and two compile jobs for this reason, and the script sets the same.
Windows installers (`assets/windows/concat.iss`, `concat.wxs`) were renamed but never built.

## If something fails

| Symptom | Likely cause |
|---|---|
| `link.exe not found` | The C++ workload of the Build Tools is missing |
| `couldn't find libclang` | `LIBCLANG_PATH` unset: run the script in this window |
| `cannot find ffmpeg` or `onnxruntime.dll` at start | The script was not run in this window, so the DLLs are not on `PATH` |
| Out of memory during the build | Close other programs; keep `CARGO_BUILD_JOBS=2` |
