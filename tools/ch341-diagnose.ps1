# CH341A 空载 ACK 行为探针
# 直接 P/Invoke CH347DLLA64.dll 导出表里的 CH341* 函数，打印真实返回值。
# 用法: pwsh -File ch341-diagnose.ps1 [-Index 0] [-DllPath <path>] [-ClockMode 1]
param(
  [int]$Index = 0,
  [string]$DllPath = '',
  [int]$ClockMode = 1
)

$ErrorActionPreference = 'Stop'

if ([Environment]::Is64BitProcess -ne $true) { Write-Host '[FATAL] 需要 64 位 PowerShell。'; exit 1 }

if ([string]::IsNullOrWhiteSpace($DllPath)) {
  $candidates = @(
    (Join-Path $PSScriptRoot '..\desktop\build-artifacts\windows-x64-ch341ack\CH347DLLA64.dll'),
    (Join-Path $PSScriptRoot '..\desktop\build-artifacts\windows-x64\CH347DLLA64.dll'),
    (Join-Path $PSScriptRoot 'CH347DLLA64.dll'),
    (Join-Path $env:SystemRoot 'System32\CH347DLLA64.dll')
  )
  $DllPath = $candidates | Where-Object { Test-Path # CH341A 空载 ACK 行为探针
# 直接 P/Invoke CH347DLLA64.dll 导出表里的 CH341* 函数，打印真实返回值。
# 用法: pwsh -File ch341-diagnose.ps1 [-Index 0] [-DllPath <path>] [-ClockMode 1]
param(
  [int]$Index = 0,
  [string]$DllPath = '',
  [int]$ClockMode = 1
)

$ErrorActionPreference = 'Stop'

if ([Environment]::Is64BitProcess -ne $true) { Write-Host '[FATAL] 需要 64 位 PowerShell。'; exit 1 }

if ([string]::IsNullOrWhiteSpace($DllPath)) {
  $candidates = @(
    'D:\重要资料勿删\My Documents\deepseek-harness\default-workspace\ek86317a-programmer-main\desktop\build-artifacts\windows-x64-ch341ack\CH347DLLA64.dll',
    'D:\重要资料勿删\My Documents\deepseek-harness\default-workspace\ek86317a-programmer-main\desktop\build-artifacts\windows-x64\CH347DLLA64.dll'
  )
  $DllPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not (Test-Path $DllPath)) { Write-Host "[FATAL] DLL 未找到: $DllPath"; exit 1 }
$DllPath = (Resolve-Path $DllPath).Path
Write-Host "DLL: $DllPath"

$src = @'
using System;
using System.Runtime.InteropServices;

public static class Ch34x {
    const string D = @"__DLL__";

    [DllImport(D, SetLastError = true)] public static extern IntPtr CH341OpenDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341CloseDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetTimeout(uint iIndex, uint w, uint r);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetStream(uint iIndex, uint iMode);
    [DllImport(D, SetLastError = true)] public static extern int CH341GetVerIC(uint iIndex, ref uint iChipType);
    [DllImport(D, SetLastError = true)] public static extern int CH341GetStatus(uint iIndex, ref uint iStatus);
    [DllImport(D, SetLastError = true)] public static extern int CH341StreamI2C(uint iIndex, uint iWriteLength, byte[] iWriteBuffer, uint iReadLength, byte[] oReadBuffer);
    [DllImport(D, SetLastError = true)] public static extern int CH341ReadI2C(uint iIndex, byte iDevice, byte iAddr, ref byte oByte);
    [DllImport(D, SetLastError = true)] public static extern int CH341WriteI2C(uint iIndex, byte iDevice, byte iAddr, byte iByte);

    [DllImport(D, SetLastError = true)] public static extern IntPtr CH347OpenDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH347CloseDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH347I2C_Set(uint iIndex, uint iMode);
    [DllImport(D, SetLastError = true)] public static extern int CH347StreamI2C(uint iIndex, uint iWriteLength, byte[] iWriteBuffer, uint iReadLength, byte[] oReadBuffer);
}
'@ -replace '__DLL__', ($DllPath -replace '\\', '\\')

Add-Type -TypeDefinition $src -Language CSharp | Out-Null
Write-Host "已加载 P/Invoke 定义`n"

function Show([string]$name, $value, [string]$extra = '') {
  Write-Host ("  {0,-42} = {1}{2}" -f $name, $value, $(if ($extra) { "   $extra" } else { '' }))
}

# ---------------------------------------------------------------- 打开设备
Write-Host '== 1. 打开 CH341 (CH341OpenDevice) =='
$h = [Ch34x]::CH341OpenDevice([uint32]$Index)
$invalid = [IntPtr](-1)
Show 'CH341OpenDevice(handle)' $h
$ch341Opened = ($h -ne $invalid -and $h -ne [IntPtr]::Zero)
if (-not $ch341Opened) { Write-Host '  [注意] CH341 设备没打开：可能没插、驱动没装、或索引不是 0。' }

Write-Host '== 2. 打开 CH347 (CH347OpenDevice) —— 检查 CH341A 是否被误认成 CH347 =='
$h347 = [Ch34x]::CH347OpenDevice([uint32]$Index)
Show 'CH347OpenDevice(handle)' $h347
$ch347Opened = ($h347 -ne $invalid -and $h347 -ne [IntPtr]::Zero)
if ($ch347Opened) { Write-Host '  [重要] CH341A 也被 CH347OpenDevice 打开成功了 → 应用会把它列成 ch347-N，走 CH347 代码路径！' }

if ($ch341Opened) {
  $ver = [uint32]0
  $okVer = [Ch34x]::CH341GetVerIC([uint32]$Index, [ref]$ver)
  Show 'CH341GetVerIC' $okVer ("chipType=0x{0:X2}" -f $ver)

  Show 'CH341SetTimeout(2000,2000)' ([Ch34x]::CH341SetTimeout([uint32]$Index, 2000, 2000))
  Show "CH341SetStream(mode=$ClockMode)" ([Ch34x]::CH341SetStream([uint32]$Index, [uint32]$ClockMode))

  $st = [uint32]0
  $okSt = [Ch34x]::CH341GetStatus([uint32]$Index, [ref]$st)
  Show 'CH341GetStatus' $okSt ("status=0x{0:X8}" -f $st)

  Write-Host "`n== 3. 空载 I2C 事务（PMIC 地址 0x20 → 写 0x40 / 读 0x41；VCOM 0x74 → 读 0xE9）=="

  # a) StreamI2C: 写 [0x41,0x00] 读 1 字节  —— 应用原来的读寄存器方式
  $w = [byte[]]@(0x41, 0x00)
  $r = [byte[]]@(0x00)
  $ok = [Ch34x]::CH341StreamI2C([uint32]$Index, [uint32]$w.Length, $w, [uint32]1, $r)
  Show 'CH341StreamI2C([0x41,0x00] → 1 byte)' $ok ("readback=0x{0:X2}" -f $r[0])

  # b) StreamI2C: 只写地址字节（address-only）
  $w2 = [byte[]]@(0x40)
  $r2 = [byte[]]@(0x00)
  $ok2 = [Ch34x]::CH341StreamI2C([uint32]$Index, [uint32]1, $w2, [uint32]0, $null)
  Show 'CH341StreamI2C([0x40] write-only)' $ok2

  # c) StreamI2C: 直接单字节读地址 0x41
  $w3 = [byte[]]@(0x41)
  $r3 = [byte[]]@(0x00)
  $ok3 = [Ch34x]::CH341StreamI2C([uint32]$Index, [uint32]1, $w3, [uint32]1, $r3)
  Show 'CH341StreamI2C([0x41] → 1 byte)' $ok3 ("readback=0x{0:X2}" -f $r3[0])

  # d) CH341ReadI2C —— 我在修复里当作 NACK 判据的那一个
  $b = [byte]0
  $okRead = [Ch34x]::CH341ReadI2C([uint32]$Index, [byte]0x41, [byte]0x00, [ref]$b)
  Show 'CH341ReadI2C(dev=0x41, reg=0x00)' $okRead ("readback=0x{0:X2}" -f $b)

  # e) CH341WriteI2C —— 写寄存器判据
  $okWrite = [Ch34x]::CH341WriteI2C([uint32]$Index, [byte]0x40, [byte]0x00, [byte]0x00)
  Show 'CH341WriteI2C(dev=0x40, reg=0x00, val=0x00)' $okWrite

  # f) 同一个读，但对 VCOM 从机 0x74 读（绝对不存在的从机地址，做对照）
  $b2 = [byte]0
  $okRead74 = [Ch34x]::CH341ReadI2C([uint32]$Index, [byte]0xE9, [byte]0x00, [ref]$b2)
  Show 'CH341ReadI2C(dev=0xE9) 对照' $okRead74 ("readback=0x{0:X2}" -f $b2)

  Write-Host "`n== 4. 结论判据 =="
  Write-Host '  · 若 StreamI2C 全 =1 且 ReadI2C/WriteI2C 也 =1 → 这版 DLL 完全不报 NACK，只能靠数据合理性判定。'
  Write-Host '  · 若 ReadI2C =0 → 修复应生效；那问题在别处（设备被识别成 ch347、或跑的不是新 exe）。'
  Write-Host '  · 若 readback 全 0x00（不是 0xFF）→ 总线没有上拉，判定阈值要同时覆盖 0x00。'

  [Ch34x]::CH341CloseDevice([uint32]$Index) | Out-Null
}

if ($ch347Opened) { [Ch34x]::CH347CloseDevice([uint32]$Index) | Out-Null }
Write-Host "`n完成。"
 } | Select-Object -First 1
}
if (-not $DllPath -or -not (Test-Path $DllPath)) { Write-Host '[FATAL] 找不到 CH347DLLA64.dll，用 -DllPath 指定路径'; exit 1 }
$DllPath = (Resolve-Path $DllPath).Path
if (-not (Test-Path $DllPath)) { Write-Host "[FATAL] DLL 未找到: $DllPath"; exit 1 }
$DllPath = (Resolve-Path $DllPath).Path
Write-Host "DLL: $DllPath"

$src = @'
using System;
using System.Runtime.InteropServices;

public static class Ch34x {
    const string D = @"__DLL__";

    [DllImport(D, SetLastError = true)] public static extern IntPtr CH341OpenDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341CloseDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetTimeout(uint iIndex, uint w, uint r);
    [DllImport(D, SetLastError = true)] public static extern int CH341SetStream(uint iIndex, uint iMode);
    [DllImport(D, SetLastError = true)] public static extern int CH341GetVerIC(uint iIndex, ref uint iChipType);
    [DllImport(D, SetLastError = true)] public static extern int CH341GetStatus(uint iIndex, ref uint iStatus);
    [DllImport(D, SetLastError = true)] public static extern int CH341StreamI2C(uint iIndex, uint iWriteLength, byte[] iWriteBuffer, uint iReadLength, byte[] oReadBuffer);
    [DllImport(D, SetLastError = true)] public static extern int CH341ReadI2C(uint iIndex, byte iDevice, byte iAddr, ref byte oByte);
    [DllImport(D, SetLastError = true)] public static extern int CH341WriteI2C(uint iIndex, byte iDevice, byte iAddr, byte iByte);

    [DllImport(D, SetLastError = true)] public static extern IntPtr CH347OpenDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH347CloseDevice(uint iIndex);
    [DllImport(D, SetLastError = true)] public static extern int CH347I2C_Set(uint iIndex, uint iMode);
    [DllImport(D, SetLastError = true)] public static extern int CH347StreamI2C(uint iIndex, uint iWriteLength, byte[] iWriteBuffer, uint iReadLength, byte[] oReadBuffer);
}
'@ -replace '__DLL__', ($DllPath -replace '\\', '\\')

Add-Type -TypeDefinition $src -Language CSharp | Out-Null
Write-Host "已加载 P/Invoke 定义`n"

function Show([string]$name, $value, [string]$extra = '') {
  Write-Host ("  {0,-42} = {1}{2}" -f $name, $value, $(if ($extra) { "   $extra" } else { '' }))
}

# ---------------------------------------------------------------- 打开设备
Write-Host '== 1. 打开 CH341 (CH341OpenDevice) =='
$h = [Ch34x]::CH341OpenDevice([uint32]$Index)
$invalid = [IntPtr](-1)
Show 'CH341OpenDevice(handle)' $h
$ch341Opened = ($h -ne $invalid -and $h -ne [IntPtr]::Zero)
if (-not $ch341Opened) { Write-Host '  [注意] CH341 设备没打开：可能没插、驱动没装、或索引不是 0。' }

Write-Host '== 2. 打开 CH347 (CH347OpenDevice) —— 检查 CH341A 是否被误认成 CH347 =='
$h347 = [Ch34x]::CH347OpenDevice([uint32]$Index)
Show 'CH347OpenDevice(handle)' $h347
$ch347Opened = ($h347 -ne $invalid -and $h347 -ne [IntPtr]::Zero)
if ($ch347Opened) { Write-Host '  [重要] CH341A 也被 CH347OpenDevice 打开成功了 → 应用会把它列成 ch347-N，走 CH347 代码路径！' }

if ($ch341Opened) {
  $ver = [uint32]0
  $okVer = [Ch34x]::CH341GetVerIC([uint32]$Index, [ref]$ver)
  Show 'CH341GetVerIC' $okVer ("chipType=0x{0:X2}" -f $ver)

  Show 'CH341SetTimeout(2000,2000)' ([Ch34x]::CH341SetTimeout([uint32]$Index, 2000, 2000))
  Show "CH341SetStream(mode=$ClockMode)" ([Ch34x]::CH341SetStream([uint32]$Index, [uint32]$ClockMode))

  $st = [uint32]0
  $okSt = [Ch34x]::CH341GetStatus([uint32]$Index, [ref]$st)
  Show 'CH341GetStatus' $okSt ("status=0x{0:X8}" -f $st)

  Write-Host "`n== 3. 空载 I2C 事务（PMIC 地址 0x20 → 写 0x40 / 读 0x41；VCOM 0x74 → 读 0xE9）=="

  # a) StreamI2C: 写 [0x41,0x00] 读 1 字节  —— 应用原来的读寄存器方式
  $w = [byte[]]@(0x41, 0x00)
  $r = [byte[]]@(0x00)
  $ok = [Ch34x]::CH341StreamI2C([uint32]$Index, [uint32]$w.Length, $w, [uint32]1, $r)
  Show 'CH341StreamI2C([0x41,0x00] → 1 byte)' $ok ("readback=0x{0:X2}" -f $r[0])

  # b) StreamI2C: 只写地址字节（address-only）
  $w2 = [byte[]]@(0x40)
  $r2 = [byte[]]@(0x00)
  $ok2 = [Ch34x]::CH341StreamI2C([uint32]$Index, [uint32]1, $w2, [uint32]0, $null)
  Show 'CH341StreamI2C([0x40] write-only)' $ok2

  # c) StreamI2C: 直接单字节读地址 0x41
  $w3 = [byte[]]@(0x41)
  $r3 = [byte[]]@(0x00)
  $ok3 = [Ch34x]::CH341StreamI2C([uint32]$Index, [uint32]1, $w3, [uint32]1, $r3)
  Show 'CH341StreamI2C([0x41] → 1 byte)' $ok3 ("readback=0x{0:X2}" -f $r3[0])

  # d) CH341ReadI2C —— 我在修复里当作 NACK 判据的那一个
  $b = [byte]0
  $okRead = [Ch34x]::CH341ReadI2C([uint32]$Index, [byte]0x41, [byte]0x00, [ref]$b)
  Show 'CH341ReadI2C(dev=0x41, reg=0x00)' $okRead ("readback=0x{0:X2}" -f $b)

  # e) CH341WriteI2C —— 写寄存器判据
  $okWrite = [Ch34x]::CH341WriteI2C([uint32]$Index, [byte]0x40, [byte]0x00, [byte]0x00)
  Show 'CH341WriteI2C(dev=0x40, reg=0x00, val=0x00)' $okWrite

  # f) 同一个读，但对 VCOM 从机 0x74 读（绝对不存在的从机地址，做对照）
  $b2 = [byte]0
  $okRead74 = [Ch34x]::CH341ReadI2C([uint32]$Index, [byte]0xE9, [byte]0x00, [ref]$b2)
  Show 'CH341ReadI2C(dev=0xE9) 对照' $okRead74 ("readback=0x{0:X2}" -f $b2)

  Write-Host "`n== 4. 结论判据 =="
  Write-Host '  · 若 StreamI2C 全 =1 且 ReadI2C/WriteI2C 也 =1 → 这版 DLL 完全不报 NACK，只能靠数据合理性判定。'
  Write-Host '  · 若 ReadI2C =0 → 修复应生效；那问题在别处（设备被识别成 ch347、或跑的不是新 exe）。'
  Write-Host '  · 若 readback 全 0x00（不是 0xFF）→ 总线没有上拉，判定阈值要同时覆盖 0x00。'

  [Ch34x]::CH341CloseDevice([uint32]$Index) | Out-Null
}

if ($ch347Opened) { [Ch34x]::CH347CloseDevice([uint32]$Index) | Out-Null }
Write-Host "`n完成。"

