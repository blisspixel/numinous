# Fast local quality gate (Windows). Use verify.ps1 for coverage, build, and artifacts.
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
Invoke-Step "docs"        {
    $savedRustdocFlags = Get-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
    try {
        $env:RUSTDOCFLAGS = "-D warnings"
        cargo doc --workspace --no-deps --locked
        if ($LASTEXITCODE -ne 0) { throw "docs failed" }
        cargo test --workspace --doc --locked
        if ($LASTEXITCODE -ne 0) { throw "doctests failed" }
    } finally {
        if ($null -ne $savedRustdocFlags) {
            $env:RUSTDOCFLAGS = $savedRustdocFlags.Value
        } else {
            Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
        }
    }
}
Invoke-Step "test"        { cargo test --workspace --all-targets --locked }
Invoke-Step "house style" { powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-style.ps1 }
Write-Host "All checks passed."
