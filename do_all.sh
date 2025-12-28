#!/bin/bash

cargo b --release --target wasm32-unknown-unknown

cp $(pwd)/target/wasm32-unknown-unknown/release/karma.wasm $(pwd)/build/karma.wasm
cp $(pwd)/karma.widl                                       $(pwd)/build/karma.widl