@echo off
call "%~dp0rotate-log.cmd"
echo Signaling running JustTheBar to restart...
"%~dp0jtb-app.exe" --restart
