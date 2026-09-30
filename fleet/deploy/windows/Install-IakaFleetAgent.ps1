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

.PARAMETER ProjectsRoot
    Racine sous laquelle vivent les depots git des sessions Claude Code (le demon y lit .git\HEAD,
    en lecture seule). Defaut : le profil utilisateur ; a restreindre (ex. %USERPROFILE%\work).

.PARAMETER IakaframePkg
    Chemin du package.json d'@naonedge/iakaframe. Defaut : deduit de iakaframe.cmd dans le PATH ;
    absent = la version d'iakaframe est publiee `null` (inconnue).

    G-EXEC-2, couche 2 : le demon est lance avec `node --permission` (Node >= 22.14) : LECTURES
    seules, ni --allow-child-process ni --allow-worker ni --allow-fs-write ; un lancement de
    processus echoue en ERR_ACCESS_DENIED. Installer avec `npm ci --omit=dev`. NON MESURE sous
    Windows (version de Node du PC non relevee ; forme des chemins/quotes a constater).

.EXAMPLE
    powershell -NoProfile -File deploy\windows\Install-IakaFleetAgent.ps1
#>
[CmdletBinding()]
param(
    [string]$TaskName = 'iakaFleetAgent',
    [string]$ProjectsRoot = $env:USERPROFILE,
    [string]$IakaframePkg = ''
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
    throw "node introuvable dans le PATH. Installez Node.js (>=22.14)."
}
$NodePath = $NodeCmd.Source
$NodeVersion = [Version]((& $NodePath --version).TrimStart('v'))
if ($NodeVersion -lt [Version]'22.14.0') {
    throw "Node $NodeVersion trop ancien : le modele de permissions (--permission) exige Node >= 22.14."
}

if (-not $IakaframePkg) {
    $Cmd = Get-Command iakaframe.cmd -ErrorAction SilentlyContinue
    if ($Cmd) {
        $Candidate = Join-Path (Split-Path $Cmd.Source) 'node_modules\@naonedge\iakaframe\package.json'
        if (Test-Path $Candidate) { $IakaframePkg = $Candidate }
    }
}

# Modele de permissions : LECTURES seules, strictement necessaires. Aucun --allow-child-process,
# --allow-worker, --allow-fs-write, --allow-addons, --allow-wasi.
$Home2  = $env:USERPROFILE
$Reads  = @(
    (Join-Path $RepoRoot 'bin'), (Join-Path $RepoRoot 'src'),
    (Join-Path $RepoRoot 'package.json'), (Join-Path $RepoRoot 'node_modules'),
    (Join-Path $Home2 '.iakafleet'), (Join-Path $Home2 '.claude\sessions'),
    (Join-Path $Home2 '.cache\iaka-agents'), $ProjectsRoot
)
if ($IakaframePkg) { $Reads += $IakaframePkg }
$PermArgs = '--permission ' + (($Reads | ForEach-Object { '"--allow-fs-read={0}"' -f $_ }) -join ' ')

Write-Host "Depot : $RepoRoot"
Write-Host "node  : $NodePath"
Write-Host "Tache : $TaskName"

$OutLog = Join-Path $LogDir 'agent.out.log'
$ErrLog = Join-Path $LogDir 'agent.err.log'

# cmd /c pour rediriger stdout/stderr vers les logs.
$EnvPrefix = ''
if ($IakaframePkg) { $EnvPrefix = 'set "IAKAFLEET_IAKAFRAME_PKG={0}" && ' -f $IakaframePkg }
$InnerCmd = '{0}"{1}" {2} "{3}" 1>> "{4}" 2>> "{5}"' -f $EnvPrefix, $NodePath, $PermArgs, $Entry, $OutLog, $ErrLog
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
