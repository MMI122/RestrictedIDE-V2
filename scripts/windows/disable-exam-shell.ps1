# Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [string]$ExamUser = "RestrictedExam",
    [switch]$RemoveMachineWidePolicies
)

$ErrorActionPreference = 'Stop'

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Run this script as Administrator."
    }
}

function Remove-RegistryValue {
    param(
        [string]$KeyPath,
        [string]$Name
    )

    if (Test-Path $KeyPath) {
        Remove-ItemProperty -Path $KeyPath -Name $Name -ErrorAction SilentlyContinue
    }
}

Assert-Admin

$user = Get-LocalUser -Name $ExamUser -ErrorAction SilentlyContinue
if (-not $user) {
    throw "Exam user '$ExamUser' does not exist on this machine."
}

$sid = $user.SID.Value
$profileDir = Join-Path "C:\Users" $ExamUser
$ntUserDat = Join-Path $profileDir "NTUSER.DAT"

if (-not (Test-Path $ntUserDat)) {
    throw "Profile hive not found at $ntUserDat"
}

$mountedHive = $false
$hiveRoot = "Registry::HKEY_USERS\$sid"

if (-not (Test-Path $hiveRoot)) {
    reg.exe load "HKU\$sid" "$ntUserDat" | Out-Null
    $mountedHive = $true
}

try {
    $shellKey = "Registry::HKEY_USERS\$sid\Software\Microsoft\Windows NT\CurrentVersion\Winlogon"
    $systemPolicyKey = "Registry::HKEY_USERS\$sid\Software\Microsoft\Windows\CurrentVersion\Policies\System"
    $explorerPolicyKey = "Registry::HKEY_USERS\$sid\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer"

    Remove-RegistryValue -KeyPath $shellKey -Name "Shell"

    Remove-RegistryValue -KeyPath $systemPolicyKey -Name "DisableTaskMgr"
    Remove-RegistryValue -KeyPath $systemPolicyKey -Name "DisableLockWorkstation"
    Remove-RegistryValue -KeyPath $systemPolicyKey -Name "DisableChangePassword"

    Remove-RegistryValue -KeyPath $explorerPolicyKey -Name "NoLogoff"
    Remove-RegistryValue -KeyPath $explorerPolicyKey -Name "NoClose"
    Remove-RegistryValue -KeyPath $explorerPolicyKey -Name "NoRun"
    Remove-RegistryValue -KeyPath $explorerPolicyKey -Name "NoControlPanel"
    Remove-RegistryValue -KeyPath $explorerPolicyKey -Name "NoViewContextMenu"
    Remove-RegistryValue -KeyPath $explorerPolicyKey -Name "NoWinKeys"

    if ($RemoveMachineWidePolicies) {
        $machineSystemPolicy = "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System"
        Remove-RegistryValue -KeyPath $machineSystemPolicy -Name "HideFastUserSwitching"
    }

    Write-Host "Exam shell mode disabled for user '$ExamUser'." -ForegroundColor Green
    Write-Host "User can sign in normally with Explorer shell again."
}
finally {
    if ($mountedHive) {
        reg.exe unload "HKU\$sid" | Out-Null
    }
}
