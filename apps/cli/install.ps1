param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\tok0"
)
$Repo = "prxm-labs/tok0"
$BaseUrl = "https://github.com/$Repo/releases"

if ($Version -eq "latest") {
    $Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $Release.tag_name.TrimStart("v")
}

$Target = "x86_64-pc-windows-msvc"
$BinaryUrl = "$BaseUrl/download/v$Version/tok0-$Target.exe"
$ShaUrl = "$BinaryUrl.sha256"

Write-Host "Installing tok0 v$Version..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$TmpFile = Join-Path $env:TEMP "tok0.exe"
Invoke-WebRequest -Uri $BinaryUrl -OutFile $TmpFile

$ExpectedHash = (Invoke-WebRequest -Uri $ShaUrl).Content.Trim().Split()[0]
$ActualHash = (Get-FileHash -Algorithm SHA256 $TmpFile).Hash.ToLower()
if ($ActualHash -ne $ExpectedHash) {
    Write-Error "SHA-256 mismatch! Aborting."
    Remove-Item $TmpFile
    exit 1
}
Move-Item -Force $TmpFile (Join-Path $InstallDir "tok0.exe")
Write-Host "Installed: $InstallDir\tok0.exe"
Write-Host "Add to PATH: `$env:PATH += `";$InstallDir`""
