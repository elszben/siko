#!/bin/bash

set -e

./build.sh

mkdir -p comp

cd siko_tester
cargo run -- ../siko ../std ../comp ../tests/success/ ../tests/fail/
