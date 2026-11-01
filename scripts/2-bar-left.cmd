@echo off
echo Bar on the left edge. Closes after 20s, or click it and press Esc.
"%~dp0jtb-app.exe" --edge left --quit-after 20
echo.
pause
