#!/bin/bash

set -e 

./build.sh

rm -rf dots

./siko simple.sk $@

./draw.sh