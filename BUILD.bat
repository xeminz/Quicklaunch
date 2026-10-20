@echo off
cd /d "%~dp0src-tauri"
echo QuickLaunch wird gebaut. Beim ersten Mal dauert das 5 bis 10 Minuten...
call cargo build --release
if not exist "target\release\quicklaunch.exe" goto fehler
set DEST=%LOCALAPPDATA%\QuickLaunch
if not exist "%DEST%" mkdir "%DEST%"
taskkill /f /im quicklaunch.exe >nul 2>&1
copy /y "target\release\quicklaunch.exe" "%DEST%\quicklaunch.exe"
start "" "%DEST%\quicklaunch.exe"
echo.
echo Fertig! QuickLaunch laeuft. Druecke Alt + Leertaste.
pause
exit /b 0
:fehler
echo.
echo FEHLGESCHLAGEN: Die Fehlermeldung steht weiter oben. Schick sie mir als Screenshot.
pause
exit /b 1
