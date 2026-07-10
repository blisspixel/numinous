# One-command verification (Windows): runs every gate and regenerates all
# artifacts into renders\. See VERIFY.md.
$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

function Step($name, $block) {
    Write-Host ""
    Write-Host "== $name ==" -ForegroundColor Cyan
    & $block
    if ($LASTEXITCODE -ne 0) { Write-Error "$name FAILED"; exit 1 }
}

Step "format" { cargo fmt --all --check }
Step "clippy" { cargo clippy --workspace --all-targets -- -D warnings }
Step "tests"  { cargo test --workspace }
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

Step "regenerate gallery into renders\" { cargo run -q --bin numinous -- gallery --dir renders --width 600 --height 600 }
Step "regenerate contact sheet" { cargo run -q --bin numinous -- contact-sheet --out renders\contact.png --cols 3 --tile 360 }
Step "regenerate lissajous audio" { cargo run -q --bin numinous -- sonify lissajous --out renders\lissajous.wav }
Step "regenerate collatz audio" { cargo run -q --bin numinous -- sonify collatz --out renders\collatz.wav }

Write-Host "`nAll checks passed." -ForegroundColor Green
Write-Host "Open renders\contact.png for the whole collection; renders\*.wav are the room sounds."
