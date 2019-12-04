
call ./build.bat
SETLOCAL EnableDelayedExpansion
@echo off
for /R "tests" %%A in ("*.sk") do echo "processing %%~fA" && .\siko %%~fA

echo on

   