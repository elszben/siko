#!/bin/bash

set -e 

./build.sh

./siko -c simple.rs simple.sk 
rustc --edition=2018 simple.rs