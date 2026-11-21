# Integrate OS — Phase 41 Portable Disk Image Generator (RAW / VDI / IMG)
param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$BuildDir = Join-Path $RepoRoot "build"
$RawPath = Join-Path $BuildDir "IntegrateOS.raw"
$ImgPath = Join-Path $BuildDir "IntegrateOS.img"
$VdiPath = Join-Path $BuildDir "IntegrateOS.vdi"
$SectorSize = 512

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "   INTEGRATE OS — PHASE 41: DISK IMAGE BUILDER (RAW/VDI) " -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# 1. Build release kernel binary
$profile = if ($Release) { "release" } else { "release" }
& (Join-Path $PSScriptRoot "build.ps1") -Release
if ($LASTEXITCODE -ne 0) {
    throw "Kernel compilation failed"
}

$kernelElf = Join-Path $RepoRoot "target\x86_64-unknown-none\$profile\integrate-kernel"
$kernelBytes = [System.IO.File]::ReadAllBytes($kernelElf)
Write-Host "  [OK] Kernel Payload: $kernelElf ($($kernelBytes.Length) bytes)"

# 2. Geometry: 64 MiB flat disk image
$totalSizeMb = 64
$totalBytes = [int64]($totalSizeMb * 1024 * 1024)
$totalSectors = [uint32]($totalBytes / $SectorSize)
Write-Host "==> Allocating $totalSizeMb MiB ($totalBytes bytes, $totalSectors sectors) disk buffer..."

$disk = New-Object byte[] $totalBytes

# 3. Master Boot Record (MBR - Sector 0)
# MBR Bootstrap code (x86 16-bit real mode stage 1 stub)
[byte[]]$mbrBootCode = @(
    0xFA,                         # cli
    0x31, 0xC0,                   # xor ax, ax
    0x8E, 0xD8,                   # mov ds, ax
    0x8E, 0xC0,                   # mov es, ax
    0x8E, 0xD0,                   # mov ss, ax
    0xBC, 0x00, 0x7C,             # mov sp, 0x7C00
    0xFB,                         # sti
    0xBE, 0x1A, 0x7C,             # mov si, msg
    0xAC,                         # lodsb
    0x08, 0xC0,                   # or al, al
    0x74, 0x06,                   # jz done
    0xB4, 0x0E,                   # mov ah, 0x0E
    0xCD, 0x10,                   # int 0x10
    0xEB, 0xF5,                   # jmp loop
    0xF4, 0xEB, 0xFD              # hlt; jmp $-1
)
[Array]::Copy($mbrBootCode, 0, $disk, 0, $mbrBootCode.Length)

# MBR Message
$mbrMsg = [System.Text.Encoding]::ASCII.GetBytes("Integrate OS MBR Bootloader`r`n`0")
[Array]::Copy($mbrMsg, 0, $disk, 0x1A, $mbrMsg.Length)

# Partition Table Entry 1 at offset 446 (0x01BE)
$partOffset = 446
$disk[$partOffset + 0] = 0x80 # Boot indicator: Active / Bootable
$disk[$partOffset + 1] = 0x00 # Starting Head: 0
$disk[$partOffset + 2] = 0x20 # Starting Sector: 32 (bits 0-5)
$disk[$partOffset + 3] = 0x21 # Starting Cylinder: 33
$disk[$partOffset + 4] = 0x83 # Partition Type: 0x83 (Linux / Native Integrate OS)
$disk[$partOffset + 5] = 0xFE # Ending Head: 254
$disk[$partOffset + 6] = 0xFF # Ending Sector / Cylinder
$disk[$partOffset + 7] = 0xFF # Ending Cylinder

# LBA 2048 alignment (1 MiB offset standard)
$partStartLba = [uint32]2048
$partSectorCount = [uint32]($totalSectors - $partStartLba)
[Array]::Copy([BitConverter]::GetBytes($partStartLba), 0, $disk, $partOffset + 8, 4)
[Array]::Copy([BitConverter]::GetBytes($partSectorCount), 0, $disk, $partOffset + 12, 4)

# MBR Signature 0x55AA at offset 510..511
$disk[510] = 0x55
$disk[511] = 0xAA

# 4. Volume Boot Record (VBR) at Partition 1 Start (LBA 2048)
$vbrOffset = [int64]($partStartLba * $SectorSize)
[Array]::Copy($mbrBootCode, 0, $disk, $vbrOffset, $mbrBootCode.Length)
$vbrMsg = [System.Text.Encoding]::ASCII.GetBytes("Integrate OS VBR Partition 1 Active`r`n`0")
[Array]::Copy($vbrMsg, 0, $disk, $vbrOffset + 0x1A, $vbrMsg.Length)
$disk[$vbrOffset + 510] = 0x55
$disk[$vbrOffset + 511] = 0xAA

# 5. Kernel Payload at LBA 2050 (offset 1,049,600)
$kernelDiskOffset = [int64](($partStartLba + 2) * $SectorSize)
[Array]::Copy($kernelBytes, 0, $disk, $kernelDiskOffset, $kernelBytes.Length)

# 6. Write RAW and IMG files
Write-Host "==> Writing Raw Disk Image to $RawPath..."
[System.IO.File]::WriteAllBytes($RawPath, $disk)
Write-Host "==> Copying Distributable IMG to $ImgPath..."
Copy-Item -Force $RawPath $ImgPath

# 7. Convert to VDI if VirtualBox is available
$vboxManage = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (Test-Path $vboxManage) {
    if (Test-Path $VdiPath) {
        Remove-Item -Force $VdiPath
    }
    Write-Host "==> Converting RAW to VirtualBox VDI ($VdiPath)..."
    & $vboxManage convertfromraw $RawPath $VdiPath --format VDI | Out-Null
    Write-Host "  [OK] VirtualBox VDI generated successfully"
}

# 8. Phase 41 Test Suite (5 Verification Tests)
Write-Host "`n[PHASE 41 TESTS] Verifying Disk Images..." -ForegroundColor Yellow

# Test 1: Geometry & Size
$rawInfo = Get-Item $RawPath
if ($rawInfo.Length -ne $totalBytes) {
    throw "RAW image size mismatch ($($rawInfo.Length) != $totalBytes)"
}
Write-Host "  [PASS] Test 1: Image Geometry verified ($totalSizeMb MiB, $totalSectors sectors)" -ForegroundColor Green

# Test 2: MBR Signature & Bootable Flag
$readDisk = [System.IO.File]::ReadAllBytes($RawPath)
$mbrSig = [BitConverter]::ToUInt16($readDisk, 510)
$bootFlag = $readDisk[446]
if ($mbrSig -ne 0xAA55 -or $bootFlag -ne 0x80) {
    throw "MBR signature or boot flag check failed (Sig=0x$($mbrSig.ToString('X4')), Boot=0x$($bootFlag.ToString('X2')))"
}
Write-Host "  [PASS] Test 2: MBR Sector 0 Signature (0x55AA) & Bootable Flag (0x80 Active) verified" -ForegroundColor Green

# Test 3: Partition 1 Alignment & Type
$partType = $readDisk[446 + 4]
$startLba = [BitConverter]::ToUInt32($readDisk, 446 + 8)
if ($partType -ne 0x83 -or $startLba -ne 2048) {
    throw "Partition 1 check failed (Type=0x$($partType.ToString('X2')), StartLBA=$startLba)"
}
Write-Host "  [PASS] Test 3: Partition 1 Type 0x83 (Linux / Integrate OS) & LBA 2048 (1 MiB align) verified" -ForegroundColor Green

# Test 4: Embedded Kernel ELF & Multiboot Header
$kernelMagic = [System.Text.Encoding]::ASCII.GetString($readDisk, [int]$kernelDiskOffset + 1, 3)
if ($kernelMagic -ne "ELF") {
    throw "Kernel payload at LBA 2050 is not ELF"
}
$mbMagic = [BitConverter]::ToUInt32($readDisk, [int]$kernelDiskOffset + 0x190)
if ($mbMagic -ne 0x1BADB002) {
    throw "Multiboot header magic not found in embedded kernel"
}
Write-Host "  [PASS] Test 4: Embedded Kernel ELF64 & Multiboot Header (0x1BADB002) verified" -ForegroundColor Green

# Test 5: Multi-Format Disk Artifacts (RAW, IMG, VDI)
$hasRaw = Test-Path $RawPath
$hasImg = Test-Path $ImgPath
$hasVdi = Test-Path $VdiPath
if (-not ($hasRaw -and $hasImg)) {
    throw "Disk artifacts missing"
}
Write-Host "  [PASS] Test 5: Multi-Format Artifacts verified (RAW: $hasRaw, IMG: $hasImg, VDI: $hasVdi)" -ForegroundColor Green

Write-Host "`n========================================================" -ForegroundColor Cyan
Write-Host "   PHASE 41: ALL 5/5 DISK IMAGE TESTS PASSED!           " -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Cyan
