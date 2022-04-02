# stdout-channel-rs

[![version](https://img.shields.io/crates/v/stdout-channel?color=blue&logo=rust&style=flat-square)](https://crates.io/crates/stdout-channel)
[![Build Status](https://github.com/ddboline/stdout-channel-rs/workflows/Rust/badge.svg?branch=main)](https://github.com/ddboline/stdout-channel-rs/actions?branch=main)
[![codecov](https://codecov.io/gh/ddboline/stdout-channel-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/ddboline/stdout-channel-rs)

Wrapper around stdout, uses deadqueue on the backend, allows for piping stdout to a vec when testing.
