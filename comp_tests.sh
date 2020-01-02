#!/bin/bash

set -e

./build.sh

TESTS=$(find tests -mindepth 1 -maxdepth 1)

for TEST in $TESTS; do
    echo "Running $TEST"
    ./siko $TEST
    OUTPUT_FILE=comp/`basename $TEST`.rs
    ./siko -c $OUTPUT_FILE $TEST
    rustfmt $OUTPUT_FILE*
done
