# Gets what building CutZone on Windows needs and prepares THIS PowerShell session for cargo.
# It mirrors the Windows job in .github/workflows/ci.yml, with the same pinned downloads
# (scripts/ci/deps.lock, checked against their digests), but keeps them in your profile and
# changes no system setting: the variables below last as long as the window does.
#
#   . .\scripts\windows-dev-env.ps1        # note the leading dot: it sets this session's variables
#
# You need first, once:  Visual Studio Build Tools with "Desktop development with C++",
# rustup, CMake and Git for Windows (which provides the bash that scripts/ci/fetch.sh runs in):
#   winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --passive"
#   winget install Rustlang.Rustup Kitware.CMake Git.Git
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$deps = Join-Path $env:USERPROFILE "cutzone-deps"
$bash = "$env:ProgramFiles\Git\bin\bash.exe"

if (-not (Test-Path $bash)) { throw "Git for Windows is needed (it provides bash): winget install Git.Git" }
foreach ($tool in "cargo", "cmake") {
  if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool is not on the path: open a new window after installing it" }
}
New-Item -ItemType Directory -Force $deps | Out-Null
Push-Location $deps
try {
  $fetch = ($root -replace "\\", "/") + "/scripts/ci/fetch.sh"
  function Fetch($name, $file) {
    & $bash $fetch $name $file
    if ($LASTEXITCODE -ne 0) { throw "the download of $name failed" }
  }

  # LLVM: bindgen needs libclang.dll. Its own folder, so an LLVM you already have is untouched.
  $llvm = Join-Path $deps "llvm"
  if (-not (Test-Path "$llvm\bin\libclang.dll")) {
    Write-Host "LLVM (about 300 MB, once)"
    Fetch "llvm-win64" "llvm.exe"
    Start-Process -Wait -FilePath .\llvm.exe -ArgumentList "/S", "/D=$llvm"
    if (-not (Test-Path "$llvm\bin\libclang.dll")) { throw "LLVM did not install to $llvm" }
    Remove-Item llvm.exe
  }

  # FFmpeg 7+ development files, which also carry ffmpeg.exe and ffprobe.exe.
  $ffmpeg = Get-ChildItem -Directory -Path (Join-Path $deps "ffmpeg-dl") -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $ffmpeg) {
    Write-Host "FFmpeg (once)"
    Fetch "ffmpeg-win64" "ffmpeg-win.zip"
    Expand-Archive ffmpeg-win.zip -DestinationPath (Join-Path $deps "ffmpeg-dl")
    Remove-Item ffmpeg-win.zip
    $ffmpeg = Get-ChildItem -Directory -Path (Join-Path $deps "ffmpeg-dl") | Select-Object -First 1
  }

  # ONNX Runtime, Microsoft's shared build, the way the Windows bundle links it.
  $ort = Join-Path $deps "onnxruntime-win-x64-1.28.0"
  if (-not (Test-Path "$ort\lib\onnxruntime.lib")) {
    Write-Host "ONNX Runtime (once)"
    Fetch "onnxruntime-win-x64" "onnxruntime.zip"
    Expand-Archive onnxruntime.zip -DestinationPath $deps
    Remove-Item onnxruntime.zip
    if (-not (Test-Path "$ort\lib\onnxruntime.lib")) { throw "$ort\lib\onnxruntime.lib is not there" }
  }
} finally {
  Pop-Location
}

$env:LIBCLANG_PATH = "$llvm\bin"
$env:FFMPEG_DIR = $ffmpeg.FullName
$env:ORT_LIB_LOCATION = "$ort\lib"
$env:ORT_PREFER_DYNAMIC_LINK = "1"
$env:PATH = "$llvm\bin;$($ffmpeg.FullName)\bin;$ort\lib;$env:PATH"
# The same two settings CI uses to keep a build inside the memory of an ordinary machine.
$env:CARGO_PROFILE_DEV_DEBUG = "0"
$env:CARGO_BUILD_JOBS = "2"

Write-Host ""
Write-Host "Ready. This session now has LIBCLANG_PATH, FFMPEG_DIR, ORT_LIB_LOCATION and the DLLs on PATH."
Write-Host "Next:  cd src; cargo build -p concat-cli"
