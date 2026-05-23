# Publishes rwatch to crates.io.
# Reads RWATCH_CARGO_CRATES_TOKEN from .env in the same directory.

$envFile = Join-Path $PSScriptRoot ".env"
if (-not (Test-Path $envFile)) {
    Write-Error ".env file not found at $envFile"
    exit 1
}

$token = $null
foreach ($line in Get-Content $envFile) {
    if ($line -match "^RWATCH_CARGO_CRATES_TOKEN=(.+)$") {
        $token = $Matches[1].Trim()
        break
    }
}

if (-not $token) {
    Write-Error "RWATCH_CARGO_CRATES_TOKEN not found in .env"
    exit 1
}

Write-Host "Publishing rwatch to crates.io..."
cargo publish --token $token
