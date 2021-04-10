# Beryl

[![Crates.io](https://img.shields.io/crates/v/beryl)](https://crates.io/crates/beryl)
[![Documentation](https://img.shields.io/docsrs/beryl)](https://docs.rs/beryl)
[![Build](https://img.shields.io/github/workflow/status/max-niederman/beryl/Build)](https://github.com/max-niederman/beryl/actions/workflows/build.yml)
[![Tests](https://img.shields.io/github/workflow/status/max-niederman/beryl/Test?label=tests)](https://github.com/max-niederman/beryl/actions/workflows/test.yml)
[![Test Coverage](https://img.shields.io/coveralls/github/max-niederman/beryl)](https://coveralls.io/github/max-niederman/beryl)
[![License](https://img.shields.io/crates/l/beryl)](./LICENSE.md)

Beryl is a format for unique identifiers. This crate implements utilities for generating these identifiers and splitting them into their component parts.

## Crystals
Beryl identifiers, or Crystals, are encoded into 64 bits as follows:
- **Generator ID**: 12-bit unsigned integer identifying the Crystal's generator. Further segmentation is
left to the application, as conflicts will not occur unless the scheme is changed unevenly over
less than a millisecond.
- **Generator Counter**: 10-bit unsigned integer incremented for every Crystal generated and
reset each millisecond.
- **Timestamp**: 42-bit unsigned integer number of milliseconds since an application-defined
epoch.

## Epochs
Beryl defines no standard epoch which a timestamp should be measured from, as the limited
timestamp size (2<sup>42</sup> milliseconds is about 140 years) may call for non-standard epochs. For
ease of use, the UNIX Epoch should be best.
