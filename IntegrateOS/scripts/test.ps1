$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

Write-Host "========================================================"
Write-Host "  INTEGRATE OS — AUTOMATED BUILD & ARTIFACT TEST SUITE  "
Write-Host "========================================================"

# 1. Verify Rust Toolchain & LLVM utilities
$sysroot = (& rustc --print sysroot).Trim()
$llvmBin = Join-Path $sysroot "lib\rustlib\x86_64-pc-windows-msvc\bin"
$rustLld = Join-Path $llvmBin "rust-lld.exe"
$llvmReadObj = Join-Path $llvmBin "llvm-readobj.exe"

if (-not (Test-Path $rustLld)) {
    throw "rust-lld.exe not found at $rustLld"
}
if (-not (Test-Path $llvmReadObj)) {
    throw "llvm-readobj.exe not found at $llvmReadObj"
}
Write-Host "  [OK] Toolchain Sysroot  : $sysroot"
Write-Host "  [OK] Bare-Metal Linker  : $rustLld"

# 2. Build & Verify Debug Kernel ELF
& (Join-Path $PSScriptRoot "build.ps1")
if ($LASTEXITCODE -ne 0) { throw "Debug build failed" }

# 3. Build & Verify Release Kernel ELF + ISO
& (Join-Path $PSScriptRoot "iso.ps1") -Release
if ($LASTEXITCODE -ne 0) { throw "Release ISO build failed" }

# 4. Inspect ELF sections with llvm-readobj
$releaseElf = Join-Path $RepoRoot "target\x86_64-unknown-none\release\integrate-kernel"
$sectionsOut = (& $llvmReadObj --sections $releaseElf) -join "`n"
if ($sectionsOut -notmatch "\.multiboot") {
    throw ".multiboot section missing in $releaseElf"
}
if ($sectionsOut -notmatch "\.text") {
    throw ".text section missing in $releaseElf"
}
Write-Host "  [OK] LLVM Section Check : Verified .multiboot and .text sections via llvm-readobj"

# 5. Verify ISO-9660 & El Torito Boot Catalog integrity
$isoPath = Join-Path $RepoRoot "build\IntegrateOS.iso"
if (-not (Test-Path $isoPath)) {
    throw "ISO image not found at $isoPath"
}
$isoBytes = [System.IO.File]::ReadAllBytes($isoPath)
$pvdMagic = [System.Text.Encoding]::ASCII.GetString($isoBytes, (16 * 2048) + 1, 5)
$brvdMagic = [System.Text.Encoding]::ASCII.GetString($isoBytes, (17 * 2048) + 7, 23)
if ($pvdMagic -ne "CD001") {
    throw "Invalid ISO-9660 PVD magic: $pvdMagic"
}
if ($brvdMagic -ne "EL TORITO SPECIFICATION") {
    throw "Invalid El Torito BRVD identifier: $brvdMagic"
}

$catOffset = 19 * 2048
[uint32]$catSum = 0
for ($i = 0; $i -lt 32; $i += 2) {
    $catSum = ($catSum + [BitConverter]::ToUInt16($isoBytes, $catOffset + $i)) -band 0xFFFF
}
if ($catSum -ne 0) {
    throw ("El Torito Boot Catalog validation checksum failed: 0x{0:X4}" -f $catSum)
}
if ($isoBytes[$catOffset + 32] -ne 0x88) {
    throw "El Torito default boot entry is not marked bootable (0x88)"
}
Write-Host "  [OK] El Torito Checksum : 0x0000 (Bootable indicator 0x88 verified)"

Write-Host "========================================================"
Write-Host "  ALL PHASE 0 BUILD, ELF64, MULTIBOOT, AND ISO TESTS PASS"
Write-Host "========================================================"
