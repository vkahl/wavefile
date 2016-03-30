Wavefile
====

[![Build Status](https://secure.travis-ci.org/ledbettj/wavefile.svg?branch=master)](https://travis-ci.org/ledbettj/wavefile)
[![Crates.io Status](http://meritbadge.herokuapp.com/wavefile)](https://crates.io/crates/wavefile)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/iron/iron/master/LICENSE)

## Overview

Wavefile is a simple crate for parsing WAV files.  It should theoretically handle any of the following:

* PCM data (most common)
* IEEE Float
* Extensible WAV files with PCM/IEEE Float data.

However, it's hard to find samples of some of these format so any feedback is appreciated :)

## Basic Example


```rust
let wav = match WaveFile::open("/home/john/test.wav") {
  Ok(w)  => w,
  Err(e) => println!("Oh no: {}", e)
}

println!("{} Hz, {} channel(s), {} total samples", w.sample_rate(), w.channels(), w.len());

for frame in w.iter() {
  // here frame is a Vec<i32> containing one value per channel in the file.
  println!("{:?}", frame);
}
```

