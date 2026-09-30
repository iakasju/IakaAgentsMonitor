#Requires -Version 5.1
<#
.SYNOPSIS
    Supprime la Tache planifiee de l'agent iakafleet. NON RECETTE.

.DESCRIPTION
    Arrete puis desenregistre la tache creee par Install-IakaFleetAgent.ps1.
    Ne touche ni au depot, ni aux logs, ni a ~\.iakafleet\.env.
    Arreter l'agent de cette facon est un arret force : le testament MQTT
    fera passer la machine "hors ligne" dans la console.
#>
[CmdletBinding()]
param(
    [string]$TaskName = 'iakaFleetAgent'
)

$ErrorActionPreference = 'Stop'

$task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if (-not $task) {
    Write-Host "Aucune tache '$TaskName' : rien a faire."
    return
}

Write-Host "Arret de la tache '$TaskName'..."
Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue

Write-Host "Suppression de la tache '$TaskName'..."
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false

Write-Host "Tache '$TaskName' supprimee. (Logs et .env conserves.)"
