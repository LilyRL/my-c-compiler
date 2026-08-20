#!/usr/bin/env sh

./writing-a-c-compiler-tests/test_compiler target/debug/my-c-compiler --bitwise --increment --compound "$@"
