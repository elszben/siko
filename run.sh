#!/bin/bash

cargo build --release

./siko tests/*.ss std/*.ss