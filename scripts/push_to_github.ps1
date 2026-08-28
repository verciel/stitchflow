
param(
    [Parameter(Mandatory=$false)]
    [string]$RepoUrl
)

$ErrorActionPreference = "Stop"

# Project workspace
$workspace = Split-Path -Parent $PSScriptRoot

# Git repository directory
$gitDir = "$env:LOCALAPPDATA\Stitchflow-git"

if (-not (Test-Path $gitDir)) {
    Write-Error "Git repository directory not found at $gitDir"
    exit 1
}

# Use the isolated Git repository
$env:GIT_DIR = $gitDir
$env:GIT_WORK_TREE = $workspace

Write-Host ""
Write-Host "Stitchflow Git Repository Status:" -ForegroundColor Cyan
git log --oneline -n 6

# Ask for repository URL if it wasn't provided
if (-not $RepoUrl) {
    Write-Host ""
    Write-Host "To push this repository to GitHub, provide your repository URL:" -ForegroundColor Yellow
    Write-Host "Example: .\scripts\push_to_github.ps1 -RepoUrl 'https://github.com/your-username/stitchflow.git'"
    Write-Host ""

    $RepoUrl = Read-Host "Enter GitHub Repository URL (or press Enter to skip)"
}

if ($RepoUrl) {

    Write-Host ""
    Write-Host "Configuring GitHub remote..." -ForegroundColor Green

    # Check whether origin already exists
    $originExists = git remote | Select-String "^origin$"

    if ($originExists) {
        Write-Host "Updating existing 'origin' remote -> $RepoUrl" -ForegroundColor Yellow
        git remote set-url origin $RepoUrl
    }
    else {
        Write-Host "Adding GitHub remote 'origin' -> $RepoUrl" -ForegroundColor Green
        git remote add origin $RepoUrl
    }

    Write-Host ""
    Write-Host "Current Git remotes:" -ForegroundColor Cyan
    git remote -v

    Write-Host ""
    Write-Host "Pushing commits to GitHub..." -ForegroundColor Green

    git push -u origin master

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Git push failed."
        exit 1
    }

    Write-Host ""
    Write-Host "Successfully pushed to GitHub!" -ForegroundColor Green
}
else {
    Write-Host ""
    Write-Host "No repository URL provided. Nothing was pushed." -ForegroundColor Yellow
}

