<#!
.SYNOPSIS
  Sets up the isolated Stitchflow development environment on Windows.

.DESCRIPTION
  Node dependencies stay in node_modules, Python embroidery-engine dependencies
  stay in .venv, and Rust uses rustup's per-user Windows toolchain because Tauri
  requires native compiler/linker integration.
#>
[CmdletBinding()]
param(
  [switch]$SkipPython,
  [switch]$SkipNode
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

function Require-Command([string]$Name, [string]$Message) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) { throw $Message }
}

if (-not $SkipNode) {
  Require-Command 'npm' 'Node.js 22+ is required. Install Node.js, then run this script again.'
  npm install
}

if (-not $SkipPython) {
  $pythonLauncher = Get-Command 'py' -ErrorAction SilentlyContinue
  $pythonCommand = Get-Command 'python' -ErrorAction SilentlyContinue
  if (-not $pythonLauncher -and -not $pythonCommand) {
    throw 'Python 3.11+ is required for the embroidery engine. Install Python, ensure it is on PATH, then run this script again.'
  }
  $venvPython = [System.IO.Path]::Combine($projectRoot, '.venv', 'Scripts', 'python.exe')
  if (-not (Test-Path $venvPython)) {
    if ($pythonLauncher) { & py -3.13 -m venv .venv } else { & python -m venv .venv }
  }
  if (-not (Test-Path $venvPython)) { throw 'Python could not create .venv. Run: py -3.13 -m venv .venv, then rerun this script.' }
  & $venvPython -m pip install --upgrade pip
  & $venvPython -m pip install -r .\src-tauri\embroidery-engine\requirements.txt
}

if (-not (Get-Command 'cargo' -ErrorAction SilentlyContinue)) {
  Write-Warning 'Rust is not installed. Install the stable MSVC Rust toolchain with rustup, reopen PowerShell, then rerun this script.'
} else {
  cargo --version
}

Write-Host 'Stitchflow development environment is ready.' -ForegroundColor Green
