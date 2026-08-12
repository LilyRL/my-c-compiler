#!/usr/bin/env sh

cargo run -- test.c --keep-intermediates
./test
echo $?
