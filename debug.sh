#!/bin/bash

set -e 

cargo build

RUST_BACKTRACE=1 cargo run -- $@