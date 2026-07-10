# House-style guard for Windows.
$ErrorActionPreference = "Stop"

$patterns = @("*.rs", "*.md", "*.toml", "*.wgsl", "*.sh", "*.ps1")
$files = @()
foreach ($pattern in $patterns) {
    $files += git ls-files -- $pattern
}
$files = $files | Sort-Object -Unique
if ($files.Count -eq 0) {
    exit 0
}

$attributionPattern = "(?i)" + "co-" + "authored-by:|" + "generated with (cla" + "ude|co" + "dex)"
$dashPattern = "[" + [regex]::Escape([string][char]0x2013) + [regex]::Escape([string][char]0x2014) + "]"
$bmpEmojiPattern = "[\u2600-\u27BF]"
$surrogateCandidatePattern = "[\uD83C-\uD83E][\uDC00-\uDFFF]"
$violations = New-Object System.Collections.Generic.List[string]

function Test-EmojiCodePoint([int]$codePoint) {
    return (($codePoint -ge 0x1F300 -and $codePoint -le 0x1FAFF) -or
        ($codePoint -ge 0x2600 -and $codePoint -le 0x27BF) -or
        $codePoint -eq 0x2728)
}

function Test-SurrogateEmoji([string]$line) {
    for ($i = 0; $i -lt $line.Length; $i++) {
        if ([char]::IsHighSurrogate($line[$i]) -and $i + 1 -lt $line.Length -and [char]::IsLowSurrogate($line[$i + 1])) {
            $codePoint = [char]::ConvertToUtf32($line[$i], $line[$i + 1])
            if (Test-EmojiCodePoint $codePoint) {
                return $true
            }
            $i++
        }
    }

    return $false
}

foreach ($file in $files) {
    $lines = [System.IO.File]::ReadAllLines($file, [System.Text.Encoding]::UTF8)
    for ($lineNumber = 0; $lineNumber -lt $lines.Length; $lineNumber++) {
        $line = $lines[$lineNumber]
        if ($line -match $dashPattern) {
            $violations.Add("${file}:$($lineNumber + 1): House-style violation (em/en dash): $line")
        }
        if ($line -match $bmpEmojiPattern -or ($line -match $surrogateCandidatePattern -and (Test-SurrogateEmoji $line))) {
            $violations.Add("${file}:$($lineNumber + 1): House-style violation (emoji): $line")
        }
        if ($line -match $attributionPattern) {
            $violations.Add("${file}:$($lineNumber + 1): House-style violation (AI/tool attribution): $line")
        }
    }
}

if ($violations.Count -ne 0) {
    foreach ($violation in $violations) {
        Write-Host $violation
    }
    Write-Host ""
    Write-Host "Fix the above before merging. House style: no emojis, no em/en dashes, no AI/tool attribution."
    exit 1
}

exit 0
