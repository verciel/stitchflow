param(
    [Parameter(Mandatory=$false)]
    [string]$RepoUrl
)

$ErrorActionPreference = "Stop"

$workspace = Split-Path -Parent $PSScriptRoot
$gitDir = "$env:LOCALAPPDATA\Stitchflow-git"

if (-not (Test-Path $gitDir)) {
    Write-Error "Git repository directory not found at $gitDir"
    exit 1
}

$env:GIT_DIR = $gitDir
$env:GIT_WORK_TREE = $workspace

Write-Host "Stitchflow Git Repository Status:" -ForegroundColor Cyan
git log --oneline -n 6

if (-not $RepoUrl) {
    Write-Host "`nTo push this repository to GitHub, provide your repository URL:" -ForegroundColor Yellow
    Write-Host "Example: .\scripts\push_to_github.ps1 -RepoUrl 'https://github.com/your-username/stitchflow.git'`n"
    $RepoUrl = Read-Host "Enter GitHub Repository URL (or press Enter to skip)"
}

if ($RepoUrl) {
    Write-Host "Adding GitHub remote 'origin' -> $RepoUrl" -ForegroundColor Green
    git remote remove origin 2>$null
    git remote add origin $RepoUrl
    
    Write-Host "Pushing commits to GitHub..." -ForegroundColor Green
    git push -u origin master
    Write-Host "Successfully pushed to GitHub!" -ForegroundColor Green
}
