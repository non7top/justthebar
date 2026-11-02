@echo off
echo Turning on the Open-Shell menu log (LogCategories=1: opening and closing of menus).
reg add "HKCU\Software\OpenShell\StartMenu\Settings" /v LogCategories /t REG_DWORD /d 1 /f
echo Now click the Start button once, then read %LOCALAPPDATA%\OpenShell\StartMenuLog.txt
echo.
pause
