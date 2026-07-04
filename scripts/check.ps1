# Local quality gate, mirroring CI (Windows). Prepends the cargo path.
# See docs/ENGINEERING.md.
$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

function Invoke-Step($name, $script) {
    Write-Host "== $name =="
    & $script
    if ($LASTEXITCODE -ne 0) { Write-Error "$name failed"; exit 1 }
}

Invoke-Step "fmt"         { cargo fmt --all --check }
Invoke-Step "clippy"      { cargo clippy --workspace --all-targets -- -D warnings }
Invoke-Step "test"        { cargo test --workspace }
Invoke-Step "house style" { bash scripts/check-style.sh }
Write-Host "All checks passed."
