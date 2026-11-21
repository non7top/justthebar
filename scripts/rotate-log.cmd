@echo off
rem Moves jtb.log to jtb.<last write date>.log and keeps the newest 5 dated logs.
setlocal enabledelayedexpansion
if exist "%~dp0jtb.log" (
  for %%f in ("%~dp0jtb.log") do set "stamp=%%~tf"
  set "stamp=!stamp:/=-!"
  set "stamp=!stamp::=-!"
  set "stamp=!stamp: =_!"
  move /y "%~dp0jtb.log" "%~dp0jtb.!stamp!.log" >nul
  for /f "skip=5 delims=" %%f in ('dir /b /o-d "%~dp0jtb.*.log"') do del "%~dp0%%f"
)
