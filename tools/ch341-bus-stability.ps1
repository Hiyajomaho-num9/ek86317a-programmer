# 总线稳定性对照：区分"真有器件应答"和"悬空/串扰残留"
param(
  [int]$Index = 0,
  [string]$DllPath = '',
  [int]$ClockMode = 1,
  [int]$Rounds = 10
)
$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($DllPath)) {
  $candidates = @(
    (Join-Path $PSScriptRoot '..\desktop\build-artifacts\windows-x64-ch341ack\CH347DLLA64.dll'),
    (Join-Path $PSScriptRoot '..\desktop\build-artifacts\windows-x64\CH347DLLA64.dll'),
    (Join-Path $PSScriptRoot 'CH347DLLA64.dll'),
    (Join-Path $env:SystemRoot 'System32\CH347DLLA64.dll')
  )
  $DllPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $DllPath -or -not (Test-Path $DllPath)) { Write-Host '[FATAL] 找不到 CH347DLLA64.dll，用 -DllPath 指定路径'; exit 1 }
$DllPath = (Resolve-Path $DllPath).Path

$src = @'
using System;
using System.Runtime.InteropServices;
public static class Ch34xStab {
    const string D = @"__DLL__";
    [DllImport(D, SetLastError = true)] public static extern IntPtr CH341OpenDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341CloseDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetTimeout(uint iIndex, uint w, uint r);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetStream(uint iIndex, uint iMode);
    [DllImport(D, SetLastError = true)] public static extern int CH341StreamI2C(uint iIndex, uint iWriteLength, byte[] iWriteBuffer, uint iReadLength, byte[] oReadBuffer);
}
'@ -replace '__DLL__', ($DllPath -replace '\\', '\\')
Add-Type -TypeDefinition $src -Language CSharp | Out-Null

$h = [Ch34xStab]::CH341OpenDevice([uint32]$Index)
if ($h -eq [IntPtr](-1) -or $h -eq [IntPtr]::Zero) { Write-Host '[FATAL] 打不开 CH341 设备'; exit 1 }
[Ch34xStab]::CH341SetTimeout([uint32]$Index, 2000, 2000) | Out-Null
[Ch34xStab]::CH341SetStream([uint32]$Index, [uint32]$ClockMode) | Out-Null

function Read-Reg([byte]$dev, [byte]$reg) {
  $w = [byte[]]@($dev, $reg)
  $r = [byte[]]@(0x00)
  $ok = [Ch34xStab]::CH341StreamI2C([uint32]$Index, [uint32]$w.Length, $w, [uint32]1, $r)
  if ($ok -eq 0) { return -1 }
  return $r[0]
}

$targets = @(
  @{ Name = 'PMIC  0x20'; Dev = [byte]0x40; Reg = [byte]0x00 },
  @{ Name = 'PMIC  0x20'; Dev = [byte]0x40; Reg = [byte]0x0A },
  @{ Name = 'VCOM  0x74'; Dev = [byte]0xE8; Reg = [byte]0x00 },
  @{ Name = '未用  0x55'; Dev = [byte]0xAA; Reg = [byte]0x00 },
  @{ Name = '保留  0x7F'; Dev = [byte]0xFE; Reg = [byte]0x00 }
)

Write-Host "每项连读 $Rounds 次（同一寄存器）：`n"
foreach ($t in $targets) {
  $vals = @()
  for ($i = 0; $i -lt $Rounds; $i++) { $vals += (Read-Reg $t.Dev $t.Reg) }
  $distinct = ($vals | Sort-Object -Unique)
  $text = ($vals | ForEach-Object { if ($_ -lt 0) { 'ERR' } else { '0x{0:X2}' -f $_ } }) -join ' '
  $stable = ($distinct.Count -eq 1)
  Write-Host ("{0} reg=0x{1:X2}  {2}" -f $t.Name, $t.Reg, $text)
  Write-Host ("        唯一值={0}  稳定={1}" -f (($distinct | ForEach-Object { '0x{0:X2}' -f $_ }) -join ','), $stable)
}

Write-Host "`n判据：稳定且不是 0xFF/0x00 → 有器件真实应答；恒 0xFF → 悬空（无器件）。"
[Ch34xStab]::CH341CloseDevice([uint32]$Index) | Out-Null

