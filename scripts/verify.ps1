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

if ($null -ne (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)) {
    Step "coverage" {
        cargo llvm-cov --workspace --fail-under-lines 80 --ignore-filename-regex 'crates[\\/](gpu|audio)[\\/]'
    }
} else {
    Write-Host "`n== coverage == (skipped: run 'cargo install cargo-llvm-cov' to enable)" -ForegroundColor Yellow
}

Write-Host "`n== house-style ==" -ForegroundColor Cyan
try {
    bash scripts/check-style.sh
    if ($LASTEXITCODE -ne 0) { Write-Warning "house-style reported issues (CI enforces this on Linux)" }
} catch {
    Write-Warning "could not run the house-style check locally (bash unavailable); CI enforces it"
}

Write-Host "`n== regenerate artifacts into renders\ ==" -ForegroundColor Cyan
cargo run -q --bin numinous -- gallery --dir renders --width 600 --height 600
cargo run -q --bin numinous -- contact-sheet --out renders\contact.png --cols 3 --tile 360
cargo run -q --bin numinous -- sonify lissajous --out renders\lissajous.wav
cargo run -q --bin numinous -- sonify collatz --out renders\collatz.wav

Write-Host "`nAll checks passed." -ForegroundColor Green
Write-Host "Open renders\contact.png for the whole collection; renders\*.wav are the room sounds."
