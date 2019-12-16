#!/bin/bash

set -e 

./build.sh

rm -rf dots

./siko simple.sk -i $@

./draw.sh