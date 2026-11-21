# Integrate OS — Phase 39 Bootable ISO Verification Suite
param(
    [string]$IsoPath = "D:\IntegrateOS\build\IntegrateOS.iso"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "   INTEGRATE OS — PHASE 39: BOOTABLE ISO TEST SUITE     " -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

if (-not (Test-Path $IsoPath)) {
    throw "ISO file not found: $IsoPath"
}

$isoBytes = [System.IO.File]::ReadAllBytes($IsoPath)
$totalBytes = $isoBytes.Length
$sectorSize = 2048

# Test 1: File Size & Sector Alignment
Write-Host "[TEST 1] Verifying ISO Size & Sector Alignment..." -ForegroundColor Yellow
if (($totalBytes % $sectorSize) -ne 0) {
    throw "ISO size ($totalBytes bytes) is not aligned to 2048-byte sector boundary"
}
$sectors = $totalBytes / $sectorSize
Write-Host "  [PASS] Size: $totalBytes bytes ($sectors sectors aligned to 2048 bytes)" -ForegroundColor Green

# Test 2: ISO-9660 Primary Volume Descriptor (PVD) at LBA 16 (offset 32768)
Write-Host "[TEST 2] Verifying ISO-9660 Primary Volume Descriptor (LBA 16)..." -ForegroundColor Yellow
$pvdOffset = 16 * $sectorSize
$vdType = $isoBytes[$pvdOffset]
$vdId = [System.Text.Encoding]::ASCII.GetString($isoBytes, $pvdOffset + 1, 5)
$volId = [System.Text.Encoding]::ASCII.GetString($isoBytes, $pvdOffset + 40, 32).Trim()
if ($vdType -ne 1 -or $vdId -ne "CD001") {
    throw "Invalid PVD at LBA 16 (Type: $vdType, Identifier: $vdId, expected 1 and 'CD001')"
}
Write-Host "  [PASS] PVD Verified: Type=$vdType, Magic='$vdId', Volume='$volId'" -ForegroundColor Green

# Test 3: El Torito Boot Record Volume Descriptor (BRVD) at LBA 17 (offset 34816)
Write-Host "[TEST 3] Verifying El Torito Boot Record (LBA 17)..." -ForegroundColor Yellow
$brvdOffset = 17 * $sectorSize
$brvdType = $isoBytes[$brvdOffset]
$brvdId = [System.Text.Encoding]::ASCII.GetString($isoBytes, $brvdOffset + 1, 5)
$bootSysId = [System.Text.Encoding]::ASCII.GetString($isoBytes, $brvdOffset + 7, 32).Trim()
$bootCatLba = [BitConverter]::ToUInt32($isoBytes, $brvdOffset + 71)
if ($brvdType -ne 0 -or $brvdId -ne "CD001" -or -not $bootSysId.StartsWith("EL TORITO")) {
    throw "Invalid El Torito BRVD at LBA 17 (Type: $brvdType, Magic='$brvdId', BootSystem='$bootSysId')"
}
Write-Host "  [PASS] BRVD Verified: System='$bootSysId', Boot Catalog LBA=$bootCatLba" -ForegroundColor Green

# Test 4: Boot Catalog Validation & Stage 2 Initial Entry (LBA 19)
Write-Host "[TEST 4] Verifying El Torito Boot Catalog & Stage 2 Initial Entry..." -ForegroundColor Yellow
$catOffset = $bootCatLba * $sectorSize
$headerId = $isoBytes[$catOffset]
$key55 = $isoBytes[$catOffset + 30]
$keyAA = $isoBytes[$catOffset + 31]
if ($headerId -ne 0x01 -or $key55 -ne 0x55 -or $keyAA -ne 0xAA) {
    throw "Boot catalog validation entry failed signature check (HeaderId=$headerId, Key=0x$($key55.ToString('X2'))$($keyAA.ToString('X2')))"
}
# Initial Default Entry (offset + 32)
$bootIndicator = $isoBytes[$catOffset + 32]
$mediaType = $isoBytes[$catOffset + 33]
$stage2Lba = [BitConverter]::ToUInt32($isoBytes, $catOffset + 40)
if ($bootIndicator -ne 0x88 -or $mediaType -ne 0x00) {
    throw "Initial boot entry is not bootable (Indicator=0x$($bootIndicator.ToString('X2')), MediaType=$mediaType)"
}
Write-Host "  [PASS] Boot Catalog: Validation Key 0x55AA, Bootable=0x88 (No Emulation), Stage2 LBA=$stage2Lba" -ForegroundColor Green

# Test 5: Kernel ELF64 Payload & Multiboot Magic Verification
Write-Host "[TEST 5] Verifying Embedded Kernel ELF64 & Multiboot Header..." -ForegroundColor Yellow
# Find stage2 code and get kernel LBA from stage2 bytes at offset 0x08
$stage2Offset = [int]($stage2Lba * $sectorSize)
$kernelLba = [BitConverter]::ToUInt32($isoBytes, $stage2Offset + 0x08)
$kernelOffset = [int]($kernelLba * $sectorSize)

$elfMagic = [System.Text.Encoding]::ASCII.GetString($isoBytes, $kernelOffset + 1, 3)
$elfClass = $isoBytes[$kernelOffset + 4] # 2 = 64-bit
$elfMachine = [BitConverter]::ToUInt16($isoBytes, $kernelOffset + 18) # 62 = x86_64
if ($elfMagic -ne "ELF" -or $elfClass -ne 2 -or $elfMachine -ne 62) {
    throw "Kernel payload at LBA $kernelLba is not a valid 64-bit x86_64 ELF binary"
}

# Scan first 8192 bytes for Multiboot 1 magic 0x1BADB002
$foundMultiboot = $false
for ($i = 0; $i -le 8192; $i += 4) {
    $magic = [BitConverter]::ToUInt32($isoBytes, $kernelOffset + $i)
    if ($magic -eq 0x1BADB002) {
        $foundMultiboot = $true
        break
    }
}
if (-not $foundMultiboot) {
    throw "Multiboot header magic 0x1BADB002 not found in first 8 KiB of kernel"
}
Write-Host "  [PASS] Kernel ELF64 at LBA ${kernelLba}: Class=64-bit, Machine=x86_64 (62), Multiboot 1 Magic 0x1BADB002 verified" -ForegroundColor Green

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "   PHASE 39: ALL 5/5 BOOTABLE ISO TESTS PASSED!         " -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Cyan
