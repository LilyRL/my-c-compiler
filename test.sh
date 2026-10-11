#!/usr/bin/env sh

cargo build
./tests/test_compiler target/debug/my-c-compiler --bitwise --increment --compound --goto --switch "$@"
