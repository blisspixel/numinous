# One-command verification (Windows): runs every gate and regenerates all
# artifacts into renders\. See VERIFY.md.
$ErrorActionPreference = "Stop"
$savedEnvironment = @{}
foreach ($name in @("Path", "NUMINOUS_JOURNEY", "NUMINOUS_SCORES", "NUMINOUS_CAIRN")) {
    $item = Get-Item "Env:$name" -ErrorAction SilentlyContinue
    $savedEnvironment[$name] = @{
        Present = $null -ne $item
        Value = if ($null -ne $item) { $item.Value } else { $null }
    }
}
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$verifyState = Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..")) ".agent\verify"
New-Item -ItemType Directory -Force $verifyState | Out-Null
$env:NUMINOUS_JOURNEY = Join-Path $verifyState "journey.txt"
$env:NUMINOUS_SCORES = Join-Path $verifyState "scores.txt"
$env:NUMINOUS_CAIRN = Join-Path $verifyState "cairn.txt"

function Step($name, $block) {
    Write-Host ""
    Write-Host "== $name ==" -ForegroundColor Cyan
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$name FAILED" }
}

try {
Step "format" { cargo fmt --all --check }
Step "clippy" { cargo clippy --workspace --all-targets -- -D warnings }
Step "tests"  { cargo test --workspace --all-targets --locked }
Step "build"  { cargo build --workspace --locked }

if ($null -ne (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) {
    Step "coverage" {
        cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex '(crates[\\/](gpu|audio)[\\/]|faces[\\/]app[\\/]src[\\/]main\.rs)'
    }
} else {
    Write-Host "`n== coverage == (skipped: run 'cargo install cargo-llvm-cov' to enable)" -ForegroundColor Yellow
}

if ($null -ne (Get-Command cargo-deny -ErrorAction SilentlyContinue)) {
    Step "supply-chain" { cargo deny check }
} else {
    Write-Host "`n== supply-chain == (skipped: run 'cargo install cargo-deny' to enable; CI enforces it)" -ForegroundColor Yellow
}

Step "house-style" { powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-style.ps1 }

Step "regenerate 349-screen app QA matrix" { cargo run -q -p numinous-app --example screens }
Step "regenerate gallery into renders\" { cargo run -q --bin numinous -- gallery --dir renders --width 600 --height 600 }
Step "regenerate contact sheet" { cargo run -q --bin numinous -- contact-sheet --out renders\contact.png --cols 3 --tile 360 }
Step "regenerate lissajous audio" { cargo run -q --bin numinous -- sonify lissajous --out renders\lissajous.wav }
Step "regenerate collatz audio" { cargo run -q --bin numinous -- sonify collatz --out renders\collatz.wav }
Step "regenerate lissajous room bed" { cargo run -q --bin numinous -- sonify lissajous --layer room-bed --out renders\lissajous-bed.wav }

Write-Host "`nAll checks passed." -ForegroundColor Green
Write-Host "Open renders\contact.png for the whole collection; lissajous-bed.wav is the room-bed PCM16 projection."
} finally {
    foreach ($name in $savedEnvironment.Keys) {
        $saved = $savedEnvironment[$name]
        if ($saved.Present) {
            Set-Item "Env:$name" $saved.Value
        } else {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        }
    }
}
