@echo off
echo Bar on the bottom edge, Explorer taskbar hidden. Closes after 30s, or click it and press Esc.
echo If the Explorer taskbar does not come back, run 9-restore-explorer.cmd
"%~dp0jtb-app.exe" --edge bottom --hide-explorer --quit-after 30
echo.
pause
