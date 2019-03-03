#!/bin/bash

set -e 

cargo build --release
./siko simple.sk std/*.sk $@
