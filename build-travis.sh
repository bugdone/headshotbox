#!/bin/bash
mkdir win
mkdir linux
python get-demoinfogo.py
mv demoinfogo.exe win/
unzip demoinfogo-linux.zip -d linux
demoinfogo_bin_win=win demoinfogo_bin_linux=linux python build.py
