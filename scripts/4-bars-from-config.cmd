@echo off
echo Bars from jtb.json next to the exe: right on monitor 0 (primary), left on monitor 1.
echo Edit and save jtb.json while it runs to see the bars move. A broken edit keeps the old layout, see jtb.log.
"%~dp0jtb-app.exe"
echo.
pause
