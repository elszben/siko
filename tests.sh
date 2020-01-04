#!/bin/bash

set -e

./build.sh

cd siko_tester
cargo run -- ../siko ../std ../comp ../tests/success/ ../tests/fail/
