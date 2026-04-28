# PowerShell install script invoked by Chocolatey during `choco install`.
# Downloads the matching prebuilt binary from GitHub Releases + verifies
# SHA-256 before dropping it into the package's tools directory — choco
# automatically puts `tools\` on PATH.

$ErrorActionPreference = 'Stop'
$packageName = 'tok0'
$toolsDir    = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$version     = '0.1.0'
$url64       = "https://github.com/prxm-labs/tok0/releases/download/v$version/tok0-x86_64-pc-windows-msvc.exe"

$installArgs = @{
  PackageName   = $packageName
  FileFullPath  = Join-Path $toolsDir "tok0.exe"
  Url64bit      = $url64
  Checksum64    = 'REPLACE_WITH_RELEASE_SHA256'
  ChecksumType64 = 'sha256'
}

Get-ChocolateyWebFile @installArgs

Write-Host "tok0 v$version installed. Run 'tok0 init' to install hooks."
