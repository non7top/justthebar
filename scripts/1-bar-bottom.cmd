@echo off
echo Bar on the bottom edge. Closes after 20s, or click it and press Esc.
"%~dp0jtb-app.exe" --edge bottom --quit-after 20
echo.
pause
