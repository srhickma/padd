#!/bin/bash
RUSTFLAGS="--cfg pcf_profile" cargo +nightly build --release
