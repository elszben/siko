#!/bin/bash

cargo build --release
./siko simple.ss std/*.ss $@
