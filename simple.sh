#!/bin/bash

cargo build --release
./siko simple.sk std/*.sk $@
