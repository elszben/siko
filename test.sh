#!/bin/bash

for FILE in `find tests -mindepth 1 -maxdepth 1`; do
    ./siko $FILE std
done
