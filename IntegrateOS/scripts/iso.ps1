param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$SectorSize = 2048

function Write-BothU16([byte[]]$Buffer, [int]$Offset, [uint16]$Value) {
    $le = [BitConverter]::GetBytes($Value)
    $Buffer[$Offset]     = $le[0]
    $Buffer[$Offset + 1] = $le[1]
    $Buffer[$Offset + 2] = $le[1]
    $Buffer[$Offset + 3] = $le[0]
}

function Write-BothU32([byte[]]$Buffer, [int]$Offset, [uint32]$Value) {
    $le = [BitConverter]::GetBytes($Value)
    $Buffer[$Offset]     = $le[0]
    $Buffer[$Offset + 1] = $le[1]
    $Buffer[$Offset + 2] = $le[2]
    $Buffer[$Offset + 3] = $le[3]
    $Buffer[$Offset + 4] = $le[3]
    $Buffer[$Offset + 5] = $le[2]
    $Buffer[$Offset + 6] = $le[1]
    $Buffer[$Offset + 7] = $le[0]
}

function Write-PaddedAscii([byte[]]$Buffer, [int]$Offset, [int]$Length, [string]$Text) {
    for ($i = 0; $i -lt $Length; $i++) {
        $Buffer[$Offset + $i] = 0x20
    }
    $ascii = [System.Text.Encoding]::ASCII.GetBytes($Text)
    $copyLen = [Math]::Min($ascii.Length, $Length)
    [Array]::Copy($ascii, 0, $Buffer, $Offset, $copyLen)
}

function Write-DirRecord([byte[]]$Buffer, [int]$Offset, [uint32]$ExtentLba, [uint32]$DataLen, [byte]$Flags, [byte[]]$NameBytes) {
    $nameLen = $NameBytes.Length
    $pad = if (($nameLen % 2) -eq 0) { 1 } else { 0 }
    $recLen = 33 + $nameLen + $pad
    $Buffer[$Offset] = [byte]$recLen
    $Buffer[$Offset + 1] = 0
    Write-BothU32 $Buffer ($Offset + 2) $ExtentLba
    Write-BothU32 $Buffer ($Offset + 10) $DataLen
    $Buffer[$Offset + 18] = 126 # 2026 - 1900
    $Buffer[$Offset + 19] = 11
    $Buffer[$Offset + 20] = 14
    $Buffer[$Offset + 25] = $Flags
    Write-BothU16 $Buffer ($Offset + 28) 1
    $Buffer[$Offset + 32] = [byte]$nameLen
    [Array]::Copy($NameBytes, 0, $Buffer, $Offset + 33, $nameLen)
    return $recLen
}

function New-Stage2ElTorito([uint32]$KernelLba, [uint32]$KernelSectors) {
    $buf = New-Object byte[] $SectorSize

    # 0x7C00: Far jump `jmp 0x0000:0x7C05` to normalize CS=0x0000
    [byte[]]$jmpFar = @(0xEA, 0x05, 0x7C, 0x00, 0x00, 0xE9, 0x78, 0x00)
    [Array]::Copy($jmpFar, 0, $buf, 0, $jmpFar.Length)

    # 0x7C08: kernel_lba & kernel_sectors
    [Array]::Copy([BitConverter]::GetBytes($KernelLba), 0, $buf, 0x08, 4)
    [Array]::Copy([BitConverter]::GetBytes($KernelSectors), 0, $buf, 0x0C, 4)
    # 0x7C14: staging_ptr = 0x01000000 (16 MiB physical staging buffer to avoid segment overlap)
    [Array]::Copy([BitConverter]::GetBytes([uint32]0x01000000), 0, $buf, 0x14, 4)

    # 0x7C20: BIOS INT 13h Disk Address Packet (DAP)
    $buf[0x20] = 0x10
    $buf[0x21] = 0x00
    [Array]::Copy([BitConverter]::GetBytes([uint16]1), 0, $buf, 0x22, 2)
    [Array]::Copy([BitConverter]::GetBytes([uint16]0x2000), 0, $buf, 0x24, 2)
    [Array]::Copy([BitConverter]::GetBytes([uint16]0x0000), 0, $buf, 0x26, 2)
    [Array]::Copy([BitConverter]::GetBytes([uint64]$KernelLba), 0, $buf, 0x28, 8)

    # 0x7C30: GDTR (limit = 23, base = 0x00007C38)
    [Array]::Copy([BitConverter]::GetBytes([uint16]23), 0, $buf, 0x30, 2)
    [Array]::Copy([BitConverter]::GetBytes([uint32]0x00007C38), 0, $buf, 0x32, 4)

    # 0x7C38: GDT (Null, 32-bit Code 0x08, 32-bit Data 0x10)
    [Array]::Copy([BitConverter]::GetBytes([uint64]0), 0, $buf, 0x38, 8)
    [Array]::Copy([BitConverter]::GetBytes([uint64]0x00CF9A000000FFFF), 0, $buf, 0x40, 8)
    [Array]::Copy([BitConverter]::GetBytes([uint64]0x00CF92000000FFFF), 0, $buf, 0x48, 8)

    # 0x7C50: Bootloader name string
    $loaderName = [System.Text.Encoding]::ASCII.GetBytes("GRUB 2.06 (IntegrateOS Multiboot)`0")
    [Array]::Copy($loaderName, 0, $buf, 0x50, $loaderName.Length)

    # 0x7C80: 16-bit Real Mode / Unreal Mode loader code
    [byte[]]$code16 = @(
        0xFA, 0xFC, 0x31, 0xC0, 0x8E, 0xD8, 0x8E, 0xC0, 0x8E, 0xD0, 0xBC, 0x00, 0x7C,
        0x88, 0x16, 0x10, 0x7C,
        0xB8, 0x01, 0x24, 0xCD, 0x15,
        0xE4, 0x92, 0x0C, 0x02, 0x24, 0xFE, 0xE6, 0x92,
        0xBF, 0x00, 0x80, 0xB9, 0x80, 0x00, 0x31, 0xC0, 0xF3, 0xAB,
        0xBF, 0x00, 0x90, 0x66, 0x31, 0xDB,
        0x66, 0xC7, 0x05, 0x14, 0x00, 0x00, 0x00,
        0x83, 0xC7, 0x04,
        0x66, 0xB8, 0x20, 0xE8, 0x00, 0x00,
        0x66, 0xBA, 0x50, 0x41, 0x4D, 0x53,
        0x66, 0xB9, 0x14, 0x00, 0x00, 0x00,
        0xCD, 0x15,
        0x72, 0x0A,
        0x83, 0xC7, 0x14,
        0x66, 0x85, 0xDB,
        0x75, 0xD8,
        0xEB, 0x03,
        0x83, 0xEF, 0x04,
        0x81, 0xEF, 0x00, 0x90,
        0x89, 0x3E, 0x2C, 0x80,
        0x66, 0xC7, 0x06, 0x30, 0x80, 0x00, 0x90, 0x00, 0x00,
        0x66, 0xC7, 0x06, 0x00, 0x80, 0x41, 0x02, 0x00, 0x00,
        0x66, 0xC7, 0x06, 0x04, 0x80, 0x80, 0x02, 0x00, 0x00,
        0x66, 0xC7, 0x06, 0x08, 0x80, 0x00, 0xFC, 0x01, 0x00,
        0x66, 0xC7, 0x06, 0x40, 0x80, 0x50, 0x7C, 0x00, 0x00,
        0x0F, 0x01, 0x16, 0x30, 0x7C,
        0x0F, 0x20, 0xC0, 0x0C, 0x01, 0x0F, 0x22, 0xC0,
        0xBB, 0x10, 0x00, 0x8E, 0xE3, 0x8E, 0xEB,
        0x24, 0xFE, 0x0F, 0x22, 0xC0,
        0x31, 0xC0, 0x8E, 0xD8, 0x8E, 0xC0,
        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
        0xC7, 0x06, 0x22, 0x7C, 0x01, 0x00,
        0xC7, 0x06, 0x24, 0x7C, 0x00, 0x20,
        0xC7, 0x06, 0x26, 0x7C, 0x00, 0x00,
        0xB4, 0x42, 0x8A, 0x16, 0x10, 0x7C, 0xBE, 0x20, 0x7C, 0xCD, 0x13,
        0x66, 0x8B, 0x3E, 0x14, 0x7C, 0xBE, 0x00, 0x20, 0xB9, 0x00, 0x02,
        0x66, 0x8B, 0x04, 0x64, 0x67, 0x66, 0x89, 0x07,
        0x83, 0xC6, 0x04, 0x66, 0x83, 0xC7, 0x04, 0x49, 0x75, 0xEE,
        0x66, 0x89, 0x3E, 0x14, 0x7C,
        0x66, 0xFF, 0x06, 0x28, 0x7C,
        0x66, 0xFF, 0x0E, 0x0C, 0x7C,
        0x75, 0xB5,
        0xFA, 0x0F, 0x01, 0x16, 0x30, 0x7C,
        0x0F, 0x20, 0xC0, 0x0C, 0x01, 0x0F, 0x22, 0xC0,
        0x66, 0xEA, 0x00, 0x7E, 0x00, 0x00, 0x08, 0x00
    )
    [Array]::Copy($code16, 0, $buf, 0x80, $code16.Length)

    # 0x7E00 (offset 0x200): 32-bit Protected Mode ELF64 segment loader & Multiboot handoff
    [byte[]]$code32 = @(
        0x66, 0xB8, 0x10, 0x00, 0x8E, 0xD8, 0x8E, 0xC0, 0x8E, 0xE0, 0x8E, 0xE8, 0x8E, 0xD0,
        0xBC, 0x00, 0x7C, 0x00, 0x00,
        0xBB, 0x00, 0x00, 0x00, 0x01,
        0x8B, 0x53, 0x20, 0x01, 0xDA,
        0x0F, 0xB7, 0x6B, 0x36,
        0x0F, 0xB7, 0x4B, 0x38,
        0x83, 0x3A, 0x01, 0x75, 0x1E,
        0x51, 0x52,
        0x8B, 0x72, 0x08, 0x81, 0xC6, 0x00, 0x00, 0x00, 0x01,
        0x8B, 0x7A, 0x18,
        0x8B, 0x4A, 0x28,
        0x57, 0x31, 0xC0, 0xF3, 0xAA, 0x5F,
        0x8B, 0x4A, 0x20, 0xF3, 0xA4,
        0x5A, 0x59,
        0x01, 0xEA, 0x49, 0x75, 0xD8,
        0x8B, 0x53, 0x18,
        0xB8, 0x02, 0xB0, 0xAD, 0x2B,
        0xBB, 0x00, 0x80, 0x00, 0x00,
        0xFF, 0xE2
    )
    [Array]::Copy($code32, 0, $buf, 0x200, $code32.Length)

    return $buf
}

# 1. Build and verify the bare-metal kernel ELF
& (Join-Path $PSScriptRoot "build.ps1") -Release:$Release
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$profileDir = if ($Release) { "release" } else { "debug" }
$kernelElfPath = Join-Path $RepoRoot "target\x86_64-unknown-none\$profileDir\integrate-kernel"
$grubCfgPath = Join-Path $RepoRoot "boot\grub\grub.cfg"
$outputIsoPath = Join-Path $RepoRoot "build\IntegrateOS.iso"

# 2. Stage ISO file tree: build/iso/boot/grub/
$stagingRoot = Join-Path $RepoRoot "build\iso"
$stagingGrub = Join-Path $stagingRoot "boot\grub"
New-Item -ItemType Directory -Force -Path $stagingGrub | Out-Null

Copy-Item -Force $grubCfgPath (Join-Path $stagingGrub "grub.cfg")
Copy-Item -Force $kernelElfPath (Join-Path $stagingRoot "boot\integrate-os.bin")

$grubCfgBytes = [System.IO.File]::ReadAllBytes($grubCfgPath)
$kernelBytes = [System.IO.File]::ReadAllBytes($kernelElfPath)

$stage2Lba = [uint32]25
$stage2Sectors = [uint32]1
$grubCfgLba = [uint32]($stage2Lba + $stage2Sectors)
$grubCfgSectors = [uint32][Math]::Max(1, [Math]::Ceiling($grubCfgBytes.Length / [double]$SectorSize))
$kernelLba = [uint32]($grubCfgLba + $grubCfgSectors)
$kernelSectors = [uint32][Math]::Max(1, [Math]::Ceiling($kernelBytes.Length / [double]$SectorSize))
$totalSectors = [uint32]($kernelLba + $kernelSectors + 2)

$iso = New-Object byte[] ($totalSectors * $SectorSize)
$stage2Bytes = New-Stage2ElTorito -KernelLba $kernelLba -KernelSectors $kernelSectors
[System.IO.File]::WriteAllBytes((Join-Path $stagingGrub "stage2_eltorito.bin"), $stage2Bytes)

$rootDirLba = [uint32]22
$bootDirLba = [uint32]23
$grubDirLba = [uint32]24

# LBA 22: Root Directory
$off = $rootDirLba * $SectorSize
$off += Write-DirRecord $iso $off $rootDirLba $SectorSize 0x02 ([byte[]]@(0))
$off += Write-DirRecord $iso $off $rootDirLba $SectorSize 0x02 ([byte[]]@(1))
$off += Write-DirRecord $iso $off $bootDirLba $SectorSize 0x02 ([System.Text.Encoding]::ASCII.GetBytes("BOOT"))

# LBA 23: /BOOT Directory
$off = $bootDirLba * $SectorSize
$off += Write-DirRecord $iso $off $bootDirLba $SectorSize 0x02 ([byte[]]@(0))
$off += Write-DirRecord $iso $off $rootDirLba $SectorSize 0x02 ([byte[]]@(1))
$off += Write-DirRecord $iso $off $grubDirLba $SectorSize 0x02 ([System.Text.Encoding]::ASCII.GetBytes("GRUB"))
$off += Write-DirRecord $iso $off $kernelLba ([uint32]$kernelBytes.Length) 0x00 ([System.Text.Encoding]::ASCII.GetBytes("INTEGRATE_OS.BIN;1"))

# LBA 24: /BOOT/GRUB Directory
$off = $grubDirLba * $SectorSize
$off += Write-DirRecord $iso $off $grubDirLba $SectorSize 0x02 ([byte[]]@(0))
$off += Write-DirRecord $iso $off $bootDirLba $SectorSize 0x02 ([byte[]]@(1))
$off += Write-DirRecord $iso $off $grubCfgLba ([uint32]$grubCfgBytes.Length) 0x00 ([System.Text.Encoding]::ASCII.GetBytes("GRUB.CFG;1"))
$off += Write-DirRecord $iso $off $stage2Lba ([uint32]$stage2Bytes.Length) 0x00 ([System.Text.Encoding]::ASCII.GetBytes("STAGE2_ELTORITO.BIN;1"))

# LBA 16: Primary Volume Descriptor
$pvd = 16 * $SectorSize
$iso[$pvd + 0] = 0x01
[Array]::Copy([System.Text.Encoding]::ASCII.GetBytes("CD001"), 0, $iso, $pvd + 1, 5)
$iso[$pvd + 6] = 0x01
Write-PaddedAscii $iso ($pvd + 8) 32 "INTEGRATE OS"
Write-PaddedAscii $iso ($pvd + 40) 32 "INTEGRATE_OS"
Write-BothU32 $iso ($pvd + 80) $totalSectors
Write-BothU16 $iso ($pvd + 120) 1
Write-BothU16 $iso ($pvd + 124) 1
Write-BothU16 $iso ($pvd + 128) ([uint16]$SectorSize)
Write-BothU32 $iso ($pvd + 132) 0
[Array]::Copy([BitConverter]::GetBytes([uint32]20), 0, $iso, $pvd + 140, 4)
$be21 = [BitConverter]::GetBytes([uint32]21)
[Array]::Reverse($be21)
[Array]::Copy($be21, 0, $iso, $pvd + 148, 4)
[void](Write-DirRecord $iso ($pvd + 156) $rootDirLba $SectorSize 0x02 ([byte[]]@(0)))
Write-PaddedAscii $iso ($pvd + 318) 128 "INTEGRATE OS ENGINEERING"
Write-PaddedAscii $iso ($pvd + 574) 128 "INTEGRATE OS ISO BUILDER"
$iso[$pvd + 881] = 0x01

# LBA 17: El Torito Boot Record Volume Descriptor
$brvd = 17 * $SectorSize
$iso[$brvd + 0] = 0x00
[Array]::Copy([System.Text.Encoding]::ASCII.GetBytes("CD001"), 0, $iso, $brvd + 1, 5)
$iso[$brvd + 6] = 0x01
[Array]::Copy([System.Text.Encoding]::ASCII.GetBytes("EL TORITO SPECIFICATION"), 0, $iso, $brvd + 7, 23)
[Array]::Copy([BitConverter]::GetBytes([uint32]19), 0, $iso, $brvd + 71, 4)

# LBA 18: Volume Descriptor Set Terminator
$term = 18 * $SectorSize
$iso[$term + 0] = 0xFF
[Array]::Copy([System.Text.Encoding]::ASCII.GetBytes("CD001"), 0, $iso, $term + 1, 5)
$iso[$term + 6] = 0x01

# LBA 19: El Torito Boot Catalog
$cat = 19 * $SectorSize
$iso[$cat + 0] = 0x01
$iso[$cat + 1] = 0x00
Write-PaddedAscii $iso ($cat + 4) 20 "INTEGRATE OS GRUB"
$iso[$cat + 30] = 0x55
$iso[$cat + 31] = 0xAA
[uint32]$sum = 0
for ($i = 0; $i -lt 32; $i += 2) {
    if ($i -ne 28) {
        $sum = ($sum + [BitConverter]::ToUInt16($iso, $cat + $i)) -band 0xFFFF
    }
}
$checksum = [uint16]((0x10000 - $sum) -band 0xFFFF)
[Array]::Copy([BitConverter]::GetBytes($checksum), 0, $iso, $cat + 28, 2)

# Initial/Default Entry at offset 32
$iso[$cat + 32] = 0x88
$iso[$cat + 33] = 0x00
[Array]::Copy([BitConverter]::GetBytes([uint16]0x07C0), 0, $iso, $cat + 34, 2)
$iso[$cat + 36] = 0x00
[Array]::Copy([BitConverter]::GetBytes([uint16]4), 0, $iso, $cat + 38, 2)
[Array]::Copy([BitConverter]::GetBytes($stage2Lba), 0, $iso, $cat + 40, 4)

# Write stage2, grub.cfg, and kernel ELF into ISO sectors
[Array]::Copy($stage2Bytes, 0, $iso, [int]($stage2Lba * $SectorSize), $stage2Bytes.Length)
[Array]::Copy($grubCfgBytes, 0, $iso, [int]($grubCfgLba * $SectorSize), $grubCfgBytes.Length)
[Array]::Copy($kernelBytes, 0, $iso, [int]($kernelLba * $SectorSize), $kernelBytes.Length)

[System.IO.File]::WriteAllBytes($outputIsoPath, $iso)
Write-Host ("==> Generated Bootable ISO: {0} ({1} bytes, {2} sectors)" -f $outputIsoPath, $iso.Length, $totalSectors)
Write-Host ("  [OK] ISO-9660 PVD       : LBA 16 (Volume 'INTEGRATE_OS')")
Write-Host ("  [OK] El Torito BRVD     : LBA 17 (Boot Catalog at LBA 19)")
Write-Host ("  [OK] El Torito Stage 2  : LBA {0} (Loads Kernel ELF at LBA {1}, {2} sectors)" -f $stage2Lba, $kernelLba, $kernelSectors)
Write-Host ("  [OK] GRUB Config        : /boot/grub/grub.cfg (LBA {0})" -f $grubCfgLba)
Write-Host ("  [OK] Kernel Image       : /boot/integrate-os.bin (LBA {0}, {1} bytes)" -f $kernelLba, $kernelBytes.Length)
