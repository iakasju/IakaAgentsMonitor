#Requires -Version 5.1
<#
.SYNOPSIS
    Enregistre l'agent iakafleet comme Tache planifiee Windows demarrant au logon.

.DESCRIPTION
    ATTENTION : NON RECETTE. Ecrit d'apres le patron iakaHub (deploy/windows), lui-meme jamais
    recette sous Windows. A executer et constater sur le PC avant de s'y fier (CA-R2, CA-R3).

    Une Tache planifiee (Register-ScheduledTask) lance `node bin\iakafleet-agent.js` au logon
    de l'utilisateur, cachee, avec redemarrage automatique sur echec. Les logs vont dans
    %USERPROFILE%\.iakafleet\logs. L'agent charge lui-meme %USERPROFILE%\.iakafleet\.env :
    aucun secret n'est transmis par ce script. Le PATH de l'utilisateur (donc `iakaframe.cmd`,
    lu par l'agent pour la version) est herite du logon.

    Ce service est l'UNIQUE lanceur de l'agent (un seul client id par machine).

.PARAMETER TaskName
    Nom de la tache planifiee. Defaut : iakaFleetAgent.

.EXAMPLE
    powershell -NoProfile -File deploy\windows\Install-IakaFleetAgent.ps1
#>
[CmdletBinding()]
param(
    [string]$TaskName = 'iakaFleetAgent'
)

$ErrorActionPreference = 'Stop'

# Le script vit dans <repo>\deploy\windows\
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Entry    = Join-Path $RepoRoot 'bin\iakafleet-agent.js'
$LogDir   = Join-Path $env:USERPROFILE '.iakafleet\logs'

if (-not (Test-Path $Entry)) {
    throw "Point d'entree introuvable : $Entry (le script doit rester dans <repo>\deploy\windows\)."
}
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}

$NodeCmd = Get-Command node -ErrorAction SilentlyContinue
if (-not $NodeCmd) {
    throw "node introuvable dans le PATH. Installez Node.js (>=20)."
}
$NodePath = $NodeCmd.Source

Write-Host "Depot : $RepoRoot"
Write-Host "node  : $NodePath"
Write-Host "Tache : $TaskName"

$OutLog = Join-Path $LogDir 'agent.out.log'
$ErrLog = Join-Path $LogDir 'agent.err.log'

# cmd /c pour rediriger stdout/stderr vers les logs.
$InnerCmd = '"{0}" "{1}" 1>> "{2}" 2>> "{3}"' -f $NodePath, $Entry, $OutLog, $ErrLog
$Action   = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument "/c $InnerCmd" -WorkingDirectory $RepoRoot
$Trigger  = New-ScheduledTaskTrigger -AtLogOn

$Settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 999 `
    -RestartInterval (New-TimeSpan -Minutes 1) `
    -ExecutionTimeLimit ([TimeSpan]::Zero) `
    -MultipleInstances IgnoreNew `
    -Hidden

$Principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Limited

if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
    Write-Host "Tache existante detectee -> remplacement."
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
}

Register-ScheduledTask -TaskName $TaskName `
    -Action $Action -Trigger $Trigger -Settings $Settings -Principal $Principal `
    -Description 'iakafleet agent (presence et etat des sessions Claude Code sur MQTT)' | Out-Null

Write-Host "Tache '$TaskName' enregistree. Demarrage immediat..."
Start-ScheduledTask -TaskName $TaskName
Get-ScheduledTask -TaskName $TaskName | Format-Table TaskName, State -AutoSize
Write-Host "Verifier : iakafleet ls (depuis une machine de la flotte)."
