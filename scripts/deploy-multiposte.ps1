# =============================================================
#  ExeOutput Studio — Script de Déploiement Multi-Postes
#  Usage: .\deploy-multiposte.ps1
# =============================================================

# ─── CONFIGURATION GLOBALE ────────────────────────────────────
$AppName       = "MonApp"                        # Nom de l'EXE (sans .exe)
$AppSource     = "\\SERVEUR\partage\MonApp"      # Dossier source partagé contenant l'EXE
$InstallDir    = "C:\MonApp"                     # Dossier d'installation sur chaque poste
$DbHost        = "192.168.1.10"                  # IP du serveur de base de données
$DbName        = "ma_base"                       # Nom de la base de données
$DbUser        = "app_user"                      # Utilisateur DB
$DbPass        = "motdepasse"                    # Mot de passe DB

# Liste des postes à déployer : Nom ou IP = Nom de la station
$Postes = @{
    "POSTE-01"  = "Caisse 1"
    "POSTE-02"  = "Caisse 2"
    "POSTE-03"  = "Accueil"
    "POSTE-04"  = "Direction"
    # Ajoutez autant de postes que nécessaire...
}
# ─────────────────────────────────────────────────────────────

# Couleurs de log
function Log-Info    { param($msg) Write-Host "  [INFO]  $msg" -ForegroundColor Cyan }
function Log-OK      { param($msg) Write-Host "  [ OK ]  $msg" -ForegroundColor Green }
function Log-Warn    { param($msg) Write-Host "  [WARN]  $msg" -ForegroundColor Yellow }
function Log-Error   { param($msg) Write-Host "  [ERREUR] $msg" -ForegroundColor Red }
function Log-Title   { param($msg) Write-Host "`n$('=' * 55)`n  $msg`n$('=' * 55)" -ForegroundColor Magenta }

# ─── DÉBUT DU DÉPLOIEMENT ─────────────────────────────────────
Log-Title "ExeOutput Studio — Déploiement Multi-Postes"
Write-Host "  Application : $AppName"
Write-Host "  Source      : $AppSource"
Write-Host "  Destination : $InstallDir"
Write-Host "  Postes cible: $($Postes.Count)"
Write-Host ""

$resultats = @()

foreach ($poste in $Postes.GetEnumerator()) {
    $nomPoste   = $poste.Key
    $nomStation = $poste.Value

    Write-Host ""
    Log-Title "Déploiement → $nomPoste ($nomStation)"

    # 1. Tester la connectivité réseau
    Log-Info "Test de connexion réseau..."
    if (-not (Test-Connection -ComputerName $nomPoste -Count 1 -Quiet)) {
        Log-Error "$nomPoste est hors ligne ou inaccessible. Ignoré."
        $resultats += [PSCustomObject]@{ Poste=$nomPoste; Station=$nomStation; Statut="❌ Hors ligne" }
        continue
    }
    Log-OK "Poste $nomPoste accessible."

    # 2. Transférer les fichiers de l'application
    $destUNC = "\\$nomPoste\C$\MonApp"
    Log-Info "Copie des fichiers vers $destUNC..."
    try {
        # Créer le dossier si nécessaire
        if (-not (Test-Path $destUNC)) {
            New-Item -ItemType Directory -Path $destUNC -Force | Out-Null
        }
        # Copier l'EXE et le dossier data (xcopy pour la robustesse)
        & xcopy "$AppSource\$AppName.exe" "$destUNC\" /Y /Q 2>&1 | Out-Null
        if (Test-Path "$AppSource\data") {
            & xcopy "$AppSource\data" "$destUNC\data\" /E /Y /Q 2>&1 | Out-Null
        }
        Log-OK "Fichiers copiés avec succès."
    } catch {
        Log-Error "Échec de la copie : $_"
        $resultats += [PSCustomObject]@{ Poste=$nomPoste; Station=$nomStation; Statut="❌ Erreur copie" }
        continue
    }

    # 3. Générer le fichier .env spécifique au poste
    Log-Info "Génération du fichier .env pour '$nomStation'..."
    $envContent = @"
# ================================================
# Configuration générée automatiquement
# Station : $nomStation — Poste : $nomPoste
# Date    : $(Get-Date -Format "yyyy-MM-dd HH:mm")
# ================================================

DB_HOST=$DbHost
DB_PORT=3306
DB_DATABASE=$DbName
DB_USERNAME=$DbUser
DB_PASSWORD=$DbPass
STATION_NAME=$nomStation
COMPUTER_NAME=$nomPoste
APP_ENV=production
APP_DEBUG=false
"@
    try {
        $envContent | Set-Content -Path "$destUNC\.env" -Encoding UTF8
        Log-OK "Fichier .env créé : STATION_NAME=$nomStation"
    } catch {
        Log-Error "Impossible d'écrire le .env : $_"
        $resultats += [PSCustomObject]@{ Poste=$nomPoste; Station=$nomStation; Statut="❌ Erreur .env" }
        continue
    }

    # 4. Créer un raccourci sur le bureau du poste
    Log-Info "Création du raccourci sur le bureau..."
    try {
        $bureauUNC  = "\\$nomPoste\C$\Users\Public\Desktop"
        $wshell     = New-Object -ComObject WScript.Shell
        $shortcut   = $wshell.CreateShortcut("$bureauUNC\$AppName.lnk")
        $shortcut.TargetPath      = "$InstallDir\$AppName.exe"
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Description     = "$AppName — $nomStation"
        $shortcut.Save()
        Log-OK "Raccourci créé sur le bureau de $nomPoste."
    } catch {
        Log-Warn "Impossible de créer le raccourci (non bloquant) : $_"
    }

    # 5. (Optionnel) Redémarrer l'application si elle était déjà ouverte
    Log-Info "Vérification si l'application est en cours d'exécution..."
    $proc = Get-Process -ComputerName $nomPoste -Name $AppName -ErrorAction SilentlyContinue
    if ($proc) {
        Log-Warn "L'application tourne sur $nomPoste. Redémarrage..."
        $proc | Stop-Process -Force
        Start-Sleep -Seconds 2
        Invoke-Command -ComputerName $nomPoste -ScriptBlock {
            param($dir, $app)
            Start-Process "$dir\$app.exe"
        } -ArgumentList $InstallDir, $AppName
        Log-OK "Application redémarrée sur $nomPoste."
    } else {
        Log-Info "Application non active sur $nomPoste (sera lancée manuellement)."
    }

    $resultats += [PSCustomObject]@{ Poste=$nomPoste; Station=$nomStation; Statut="✅ Succès" }
}

# ─── RAPPORT FINAL ────────────────────────────────────────────
Write-Host ""
Log-Title "RAPPORT DE DÉPLOIEMENT"
$resultats | Format-Table -AutoSize

$succes = ($resultats | Where-Object { $_.Statut -like "✅*" }).Count
$echecs = ($resultats | Where-Object { $_.Statut -like "❌*" }).Count

Write-Host "  ✅ $succes poste(s) déployé(s) avec succès." -ForegroundColor Green
if ($echecs -gt 0) {
    Write-Host "  ❌ $echecs poste(s) en échec." -ForegroundColor Red
}
Write-Host ""
Write-Host "Déploiement terminé. Appuyez sur une touche pour quitter..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
