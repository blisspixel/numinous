# Reference-machine performance gate for the five 0.3 flagships.
$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

cargo run --release --locked -p numinous-app --example flagship_perf -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Error "flagship performance gate failed"
    exit 1
}
