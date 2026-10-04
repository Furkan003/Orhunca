# Orhunca'yı (orhunca komutu) Windows'a kurar; yönetici izni gerekmez.
#   irm https://furkan003.github.io/orhunca/kur.ps1 | iex
# Kurulum yeri: %LOCALAPPDATA%\Orhunca; kullanıcının PATH'ine eklenir.
# Stüdyo dahil tam kurulum için sitedeki "Windows kurulum dosyası"nı kullanın.
$ErrorActionPreference = 'Stop'
$Depo = 'Furkan003/Orhunca'
$Hedef = Join-Path $env:LOCALAPPDATA 'Orhunca'
$Adres = "https://github.com/$Depo/releases/latest/download/orhunca-windows-x86_64.zip"
$Gecici = Join-Path ([IO.Path]::GetTempPath()) ("orhunca-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $Gecici, $Hedef | Out-Null
try {
    Write-Host "İndiriliyor: $Adres"
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -UseBasicParsing -Uri $Adres -OutFile (Join-Path $Gecici 'orhunca.zip')
    Expand-Archive -Force -Path (Join-Path $Gecici 'orhunca.zip') -DestinationPath $Gecici
    Copy-Item -Force (Join-Path $Gecici 'orhunca.exe') (Join-Path $Hedef 'orhunca.exe')
} finally {
    Remove-Item -Recurse -Force $Gecici -ErrorAction SilentlyContinue
}
$Yol = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not $Yol) { $Yol = '' }
if (-not ($Yol.Split(';') -contains $Hedef)) {
    [Environment]::SetEnvironmentVariable('Path', ($Yol.TrimEnd(';') + ';' + $Hedef).TrimStart(';'), 'User')
    $env:Path = "$env:Path;$Hedef"
}
Write-Host "Kuruldu: $Hedef\orhunca.exe"
& (Join-Path $Hedef 'orhunca.exe') sürüm
Write-Host 'Başlamak için yeni bir terminalde: orhunca stüdyo'
