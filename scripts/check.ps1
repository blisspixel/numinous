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
Invoke-Step "MCP play driver" { python scripts/test-mcp-play.py }
Invoke-Step "understanding study runner" { python scripts/test-understanding-study.py }
Invoke-Step "understanding study collector" { python scripts/test-understanding-collect.py }
Invoke-Step "release packaging" { python scripts/test-package-release.py }
Invoke-Step "release engagement contract" { python scripts/test-release-engagement-smoke.py }
Invoke-Step "physical input session contract" { python scripts/test-input-hardware-session.py }
Invoke-Step "release SBOM contract" { python scripts/test-release-sbom.py }
Invoke-Step "release workflow contract" { python scripts/test-release-workflow.py }
Invoke-Step "dependency migration performance contract" { python scripts/test-dependency-migration-performance.py }
Invoke-Step "dependency migration performance receipt" { python scripts/dependency-migration-performance.py --verify-receipt docs/evidence/dependency-migration-2026-08-02.json }
Invoke-Step "house style" { powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check-style.ps1 }
Write-Host "All checks passed."
