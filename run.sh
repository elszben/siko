#!/bin/bash

cargo build --release

./siko tests/*.sk std/*.sk