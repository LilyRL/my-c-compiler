#!/usr/bin/env sh

cargo run -- test.c --keep-intermediates -lm
./test
echo "return code: $?"
