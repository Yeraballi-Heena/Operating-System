param(
    [string]$VmName = "IntegrateOS_D",
    [switch]$Headless,
    [int]$WaitSeconds = 5,
    [switch]$PowerOffAfterWait
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$IsoPath = Join-Path $RepoRoot "build\IntegrateOS.iso"
$SerialLog = Join-Path $RepoRoot "build\serial.log"
$ScreenshotPath = Join-Path $RepoRoot "build\vbox_screen.png"
$VBoxDir = Join-Path $RepoRoot "build\vbox"
$VBoxManage = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"

if (-not (Test-Path $VBoxManage)) {
    throw "VBoxManage.exe not found at $VBoxManage"
}

if (-not (Test-Path $IsoPath)) {
    Write-Host "==> ISO not found at $IsoPath; building ISO first..."
    & (Join-Path $PSScriptRoot "iso.ps1") -Release
}

New-Item -ItemType Directory -Force -Path $VBoxDir | Out-Null

$ErrorActionPreference = "Continue"

# Check if VM already exists; if running, power it off cleanly before reconfiguring media
$existingVms = & $VBoxManage list vms
if ($existingVms -match "`"$VmName`"") {
    $runningVms = & $VBoxManage list runningvms
    if ($runningVms -match "`"$VmName`"") {
        & $VBoxManage controlvm $VmName poweroff | Out-Null
        Start-Sleep -Seconds 2
    }
} else {
    Write-Host "==> Creating 64-bit VirtualBox VM '$VmName'..."
    & $VBoxManage createvm --name $VmName --ostype "Other_64" --basefolder $VBoxDir --register
    & $VBoxManage modifyvm $VmName --memory 512 --vram 64 --cpus 1 --boot1 dvd --boot2 none --boot3 none --boot4 none
    & $VBoxManage modifyvm $VmName --ioapic on --rtcuseutc on --audio-enabled off --graphicscontroller vboxvga
    & $VBoxManage storagectl $VmName --name "IDE Controller" --add ide --controller PIIX4
}

# Ensure existing VM gets updated display VRAM, RAM, and graphics controller
& $VBoxManage modifyvm $VmName --memory 512 --vram 64 --graphicscontroller vboxvga

if (Test-Path $SerialLog) {
    Remove-Item -Force $SerialLog
}

$DiskPath = Join-Path $VBoxDir "integrate_disk.vdi"
if (-not (Test-Path $DiskPath)) {
    Write-Host "==> Creating 2048 MiB (2 GB) virtual SSD disk at $DiskPath..."
    & $VBoxManage createmedium disk --filename $DiskPath --size 2048 --format VDI | Out-Null
}

& $VBoxManage modifyvm $VmName --mouse ps2 --keyboard ps2 --uart1 0x3F8 4 --uartmode1 file $SerialLog
& $VBoxManage setextradata global GUI/SuppressMessages "remindAboutAutoCapture,remindAboutMouseIntegration,confirmGoingFullscreen,remindAboutWrongColorDepth" | Out-Null
& $VBoxManage setextradata $VmName "GUI/ScaleFactor" "1.5" | Out-Null
& $VBoxManage setextradata $VmName "CustomVideoMode1" "1024x768x32" | Out-Null
& $VBoxManage storageattach $VmName --storagectl "IDE Controller" --port 0 --device 0 --type dvddrive --medium $IsoPath
& $VBoxManage storageattach $VmName --storagectl "IDE Controller" --port 0 --device 1 --type hdd --medium $DiskPath

$launchType = if ($Headless) { "headless" } else { "gui" }
Write-Host "==> Starting VirtualBox VM '$VmName' ($launchType mode)..."
& $VBoxManage startvm $VmName --type $launchType

if ($WaitSeconds -gt 0) {
    Start-Sleep -Seconds $WaitSeconds
    & $VBoxManage controlvm $VmName screenshotpng $ScreenshotPath | Out-Null
    if (Test-Path $SerialLog) {
        Write-Host "`n==> Serial Output ($SerialLog):"
        Get-Content $SerialLog
    }
    if ($PowerOffAfterWait) {
        & $VBoxManage controlvm $VmName poweroff | Out-Null
    }
}
