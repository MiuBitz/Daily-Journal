@echo off
setlocal
cd /d "%~dp0"
cargo --version >nul 2>&1
if errorlevel 1 (
  echo Rust is required. Install Rust and Microsoft C++ Build Tools first.
  exit /b 1
)
set "JOURNAL_BUILD_DIR=%~dp0target"
if defined CARGO_TARGET_DIR set "JOURNAL_BUILD_DIR=%CARGO_TARGET_DIR%"
echo Building Daily Journal 1.0...
cargo build --release --locked
if errorlevel 1 exit /b 1
if not exist "dist" mkdir "dist"
copy /y "%JOURNAL_BUILD_DIR%\release\daily_journal.exe" "dist\DailyJournal.exe" >nul
if errorlevel 1 (
  echo Could not replace the executable. Exit Daily Journal from its tray menu and try again.
  exit /b 1
)
echo Ready: %CD%\dist\DailyJournal.exe
