# Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$AppPath,

    [string]$ExamUser = "RestrictedExam",

    [string]$ExamUserPassword = "ExamMode!123",

    [switch]$ApplyMachineWidePolicies
)

$ErrorActionPreference = 'Stop'

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "Run this script as Administrator."
    }
}

function Ensure-LocalUser {
    param(
        [string]$UserName,
        [string]$Password
    )

    $user = Get-LocalUser -Name $UserName -ErrorAction SilentlyContinue
    if (-not $user) {
        $secure = ConvertTo-SecureString $Password -AsPlainText -Force
        New-LocalUser -Name $UserName -Password $secure -PasswordNeverExpires -AccountNeverExpires | Out-Null
        Add-LocalGroupMember -Group "Users" -Member $UserName -ErrorAction SilentlyContinue
        Write-Host "Created local exam user: $UserName"
    } else {
        if (-not $user.Enabled) {
            Enable-LocalUser -Name $UserName
        }
        Write-Host "Using existing local exam user: $UserName"
    }
}

function Get-UserSid {
    param([string]$UserName)

    $u = Get-LocalUser -Name $UserName -ErrorAction Stop
    return $u.SID.Value
}

function Ensure-ProfileHiveAvailable {
    param([string]$UserName)

    $profileDir = Join-Path "C:\Users" $UserName
    $ntUserDat = Join-Path $profileDir "NTUSER.DAT"

    if (-not (Test-Path $ntUserDat)) {
        throw "User profile hive not found at $ntUserDat. Sign in once with '$UserName', then sign out and rerun."
    }

    return $ntUserDat
}

function Set-RegistryValue {
    param(
        [string]$KeyPath,
        [string]$Name,
        [Parameter(Mandatory = $true)]
        $Value,
        [ValidateSet('String', 'DWord')]
        [string]$Type = 'DWord'
    )

    if (-not (Test-Path $KeyPath)) {
        New-Item -Path $KeyPath -Force | Out-Null
    }

    $propertyType = if ($Type -eq 'String') { 'String' } else { 'DWord' }
    New-ItemProperty -Path $KeyPath -Name $Name -Value $Value -PropertyType $propertyType -Force | Out-Null
}

Assert-Admin

if (-not (Test-Path $AppPath)) {
    throw "AppPath does not exist: $AppPath"
}

Ensure-LocalUser -UserName $ExamUser -Password $ExamUserPassword
$sid = Get-UserSid -UserName $ExamUser
$ntUserDat = Ensure-ProfileHiveAvailable -UserName $ExamUser

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

    $shellValue = '"' + $AppPath + '" --kiosk --exam-shell'

    Set-RegistryValue -KeyPath $shellKey -Name "Shell" -Value $shellValue -Type String

    Set-RegistryValue -KeyPath $systemPolicyKey -Name "DisableTaskMgr" -Value 1
    Set-RegistryValue -KeyPath $systemPolicyKey -Name "DisableLockWorkstation" -Value 1
    Set-RegistryValue -KeyPath $systemPolicyKey -Name "DisableChangePassword" -Value 1

    Set-RegistryValue -KeyPath $explorerPolicyKey -Name "NoLogoff" -Value 1
    Set-RegistryValue -KeyPath $explorerPolicyKey -Name "NoClose" -Value 1
    Set-RegistryValue -KeyPath $explorerPolicyKey -Name "NoRun" -Value 1
    Set-RegistryValue -KeyPath $explorerPolicyKey -Name "NoControlPanel" -Value 1
    Set-RegistryValue -KeyPath $explorerPolicyKey -Name "NoViewContextMenu" -Value 1
    Set-RegistryValue -KeyPath $explorerPolicyKey -Name "NoWinKeys" -Value 1

    if ($ApplyMachineWidePolicies) {
        $machineSystemPolicy = "Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System"
        Set-RegistryValue -KeyPath $machineSystemPolicy -Name "HideFastUserSwitching" -Value 1
    }

    Write-Host ""
    Write-Host "Exam shell mode enabled for user '$ExamUser' (SID: $sid)." -ForegroundColor Green
    Write-Host "Shell: $shellValue"
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "1. Sign in as '$ExamUser'."
    Write-Host "2. Restricted IDE should launch as the shell."
    Write-Host "3. After exams, run disable-exam-shell.ps1 from an admin account."
}
finally {
    if ($mountedHive) {
        reg.exe unload "HKU\$sid" | Out-Null
    }
}
