#!/bin/bash

./build.sh

TESTS=$(find tests -mindepth 1 -maxdepth 1)

for TEST in $TESTS; do
    echo "Running $TEST"
    ./siko $TEST
done
