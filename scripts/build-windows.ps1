$ErrorActionPreference = "Stop"
$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RootDir

Write-Host "==> 1. Running workspace release tests & latency-contract benchmarks"
cargo test --workspace --release
cargo run --release -p klink-cli --bin klink-bench

Write-Host "==> 2. Building release CLI binary (klink-cli.exe)"
cargo build --release -p klink-cli

Write-Host "==> 3. Building Windows Desktop MSI & NSIS EXE via Tauri v2"
Set-Location (Join-Path $RootDir "apps\desktop\ui")
npm install
Set-Location (Join-Path $RootDir "apps\desktop")
.\ui\node_modules\.bin\tauri.cmd build

$DistDir = Join-Path $RootDir "dist\windows"
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
Copy-Item (Join-Path $RootDir "target\release\klink-cli.exe") $DistDir -Force
Get-ChildItem -Path (Join-Path $RootDir "apps\desktop\src-tauri\target\release\bundle") -Recurse -Include *.msi,*.exe | ForEach-Object {
    Copy-Item $_.FullName $DistDir -Force
}
Write-Host "==> Done! Windows release artifacts in dist\windows\"
Get-ChildItem $DistDir
