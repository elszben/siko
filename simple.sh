#!/bin/bash

set -e 

cargo run -- simple.sk std/*.sk $@
