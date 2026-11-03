@echo off
echo Turning off the Open-Shell menu log.
reg delete "HKCU\Software\OpenShell\StartMenu\Settings" /v LogCategories /f
echo.
pause
