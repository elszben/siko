#!/bin/bash

set -e 

./build.sh

./siko simple.sk $@
