param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

function Test-KernelElf {
    param([string]$ElfPath)

    if (-not (Test-Path $ElfPath)) {
        throw "Kernel ELF binary not found at: $ElfPath"
    }

    $bytes = [System.IO.File]::ReadAllBytes($ElfPath)
    if ($bytes.Length -lt 64) {
        throw "File is too small to be a valid ELF64 binary (< 64 bytes)"
    }

    # 1. Verify ELF Magic (\x7FELF)
    if ($bytes[0] -ne 0x7F -or $bytes[1] -ne 0x45 -or $bytes[2] -ne 0x4C -or $bytes[3] -ne 0x46) {
        throw "Invalid ELF magic in $ElfPath"
    }

    # 2. Verify 64-bit (EI_CLASS = 2) and Little-Endian (EI_DATA = 1)
    if ($bytes[4] -ne 2 -or $bytes[5] -ne 1) {
        throw "Expected 64-bit LSB ELF binary (EI_CLASS=$($bytes[4]), EI_DATA=$($bytes[5]))"
    }

    # 3. Verify ET_EXEC (2) and EM_X86_64 (62)
    $eType = [BitConverter]::ToUInt16($bytes, 16)
    $eMachine = [BitConverter]::ToUInt16($bytes, 18)
    $eEntry = [BitConverter]::ToUInt64($bytes, 24)
    $ePhoff = [BitConverter]::ToUInt64($bytes, 32)
    $ePhentsize = [BitConverter]::ToUInt16($bytes, 54)
    $ePhnum = [BitConverter]::ToUInt16($bytes, 56)

    if ($eType -ne 2) {
        throw "Expected static executable ELF (ET_EXEC=2), found $eType"
    }
    if ($eMachine -ne 62) {
        throw "Expected x86_64 architecture (EM_X86_64=62), found $eMachine"
    }
    if ($eEntry -lt 0x100000) {
        throw ("Expected kernel entry point >= 0x100000, found 0x{0:X}" -f $eEntry)
    }

    # 4. Count PT_LOAD segments
    $loadSegments = 0
    for ($i = 0; $i -lt $ePhnum; $i++) {
        $off = [int]$ePhoff + ($i * $ePhentsize)
        $pType = [BitConverter]::ToUInt32($bytes, $off)
        if ($pType -eq 1) {
            $loadSegments++
        }
    }
    if ($loadSegments -eq 0) {
        throw "ELF64 binary has no PT_LOAD segments"
    }

    # 5. Scan first 8192 bytes for Multiboot1 header (0x1BADB002)
    $searchLimit = [Math]::Min($bytes.Length, 8192)
    $multibootOffset = -1
    for ($off = 0; ($off + 12) -le $searchLimit; $off += 4) {
        $magic = [BitConverter]::ToUInt32($bytes, $off)
        if ($magic -eq 0x1BADB002) {
            $flags = [BitConverter]::ToUInt32($bytes, $off + 4)
            $checksum = [BitConverter]::ToUInt32($bytes, $off + 8)
            $sum = ([int64]$magic + [int64]$flags + [int64]$checksum) -band 0xFFFFFFFFL
            if ($sum -eq 0) {
                $multibootOffset = $off
                break
            }
        }
    }

    if ($multibootOffset -lt 0) {
        throw "Multiboot1 header (0x1BADB002) with valid checksum not found within first 8192 bytes"
    }

    Write-Host ("  [OK] ELF64 Binary     : {0} ({1} bytes)" -f $ElfPath, $bytes.Length)
    Write-Host ("  [OK] Architecture     : 64-bit LSB ET_EXEC, EM_X86_64 (62)")
    Write-Host ("  [OK] Entry Point      : 0x{0:X8}" -f $eEntry)
    Write-Host ("  [OK] PT_LOAD Segments : {0}" -f $loadSegments)
    Write-Host ("  [OK] Multiboot Header : Verified at file offset 0x{0:X4} (< 8192 bytes)" -f $multibootOffset)
}

Push-Location $RepoRoot
try {
    $profileName = if ($Release) { "release" } else { "dev (debug)" }
    $profileDir = if ($Release) { "release" } else { "debug" }
    $elfPath = Join-Path $RepoRoot "target\x86_64-unknown-none\$profileDir\integrate-kernel"
    $linkerScript = Join-Path $RepoRoot "kernel\linker.ld"
    $mainRs = Join-Path $RepoRoot "kernel\src\main.rs"

    if ((Test-Path $elfPath) -and ((Get-Item $linkerScript).LastWriteTimeUtc -gt (Get-Item $elfPath).LastWriteTimeUtc)) {
        (Get-Item $mainRs).LastWriteTimeUtc = [DateTime]::UtcNow
    }

    Write-Host "==> Building integrate-kernel [$profileName] for target x86_64-unknown-none..."

    if ($Release) {
        cargo build --package integrate-kernel --target x86_64-unknown-none --release
    } else {
        cargo build --package integrate-kernel --target x86_64-unknown-none
    }
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }

    Test-KernelElf -ElfPath $elfPath
} finally {
    Pop-Location
}
