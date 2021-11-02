#!/bin/sh

rm -rf ./target *.prof*

export RUSTFLAGS="-Zinstrument-coverage"

cargo +nightly build

cargo +nightly test --lib

grcov . --binary-path ./target/debug -s . -t html --branch --ignore-not-existing -o ./target/debug/coverage/ --excl-br-line "#\[derive\(" --excl-line "#\[derive\("

