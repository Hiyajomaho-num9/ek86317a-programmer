# 用真实 DLL 走一遍新探测逻辑（与 src-tauri/src/pmu/detect.rs 的判定一致）
# 空载时应当输出 "PMIC: missing"。
param(
  [int]$Index = 0,
  [string]$DllPath = '',
  [int]$ClockMode = 1
)
$ErrorActionPreference = 'Stop'
if ([Environment]::Is64BitProcess -ne $true) { Write-Host '[FATAL] 需要 64 位 PowerShell'; exit 1 }

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
public static class Ch34xSim {
    const string D = @"__DLL__";
    [DllImport(D, SetLastError = true)] public static extern IntPtr CH341OpenDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341CloseDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetTimeout(uint iIndex, uint w, uint r);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetStream(uint iIndex, uint iMode);
    [DllImport(D, SetLastError = true)] public static extern int CH341StreamI2C(uint iIndex, uint iWriteLength, byte[] iWriteBuffer, uint iReadLength, byte[] oReadBuffer);
}
'@ -replace '__DLL__', ($DllPath -replace '\\', '\\')
Add-Type -TypeDefinition $src -Language CSharp | Out-Null

function Read-Reg([int]$dev, [byte]$reg) {
  $w = [byte[]]@([byte]$dev, $reg)
  $r = [byte[]]@(0x00)
  $ok = [Ch34xSim]::CH341StreamI2C([uint32]$Index, [uint32]$w.Length, $w, [uint32]1, $r)
  [pscustomobject]@{ Ok = ($ok -ne 0); Value = $r[0] }
}

$h = [Ch34xSim]::CH341OpenDevice([uint32]$Index)
if ($h -eq [IntPtr](-1) -or $h -eq [IntPtr]::Zero) { Write-Host '[FATAL] 打不开 CH341 设备'; exit 1 }
[Ch34xSim]::CH341SetTimeout([uint32]$Index, 2000, 2000) | Out-Null
[Ch34xSim]::CH341SetStream([uint32]$Index, [uint32]$ClockMode) | Out-Null

Write-Host '== 模拟应用 probe：先选 DAC 页 (write 0x20 <- [0xFF,0x00]) =='
$sel = [byte[]]@(0x40, 0xFF, 0x00)
$okSel = [Ch34xSim]::CH341StreamI2C([uint32]$Index, [uint32]$sel.Length, $sel, [uint32]0, $null)
Write-Host ("  bank select -> {0}" -f $(if ($okSel -ne 0) { 'TRUE' } else { 'FALSE' }))
Start-Sleep -Milliseconds 5

Write-Host '== 悬空基线：保留地址 0x7F 读寄存器 0x00 =='
$b = Read-Reg 0xFE 0x00
Write-Host ("  0x7F/reg0x00 -> Ok={0} value=0x{1:X2}" -f $b.Ok, $b.Value)

Write-Host '== 目标 PMIC 0x20 的三个探测寄存器：0x00 / 0x0A / 0x0B =='
$values = @()
foreach ($reg in 0x00, 0x0A, 0x0B) {
  $v = Read-Reg 0x40 $reg
  Write-Host ("  0x20/reg0x{0:X2} -> Ok={1} value=0x{2:X2}" -f $reg, $v.Ok, $v.Value)
  $values += $v.Value
}

$baseline = $b.Value
$present = ($baseline -eq 0xFF -or $baseline -eq 0x00) -and ($values | Where-Object { $_ -ne $baseline }).Count -gt 0
Write-Host ''
Write-Host ("baseline=0x{0:X2}  reads=[{1}]" -f $baseline, (($values | ForEach-Object { '0x{0:X2}' -f $_ }) -join ', '))
Write-Host ("判定: PMIC = {0}" -f $(if ($present) { 'ok (present)' } else { 'missing' }))
[Ch34xSim]::CH341CloseDevice([uint32]$Index) | Out-Null

