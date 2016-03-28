extern crate byteorder;
extern crate memmap;
#[macro_use]
extern crate nom;

mod parser;
pub mod error;

pub use self::error::WaveError;

use std::io::{Seek,SeekFrom,Cursor};
use memmap::{Mmap,Protection};

use byteorder::{LittleEndian, ReadBytesExt};

const RIFF : u32 = 0x46464952;
const WAVE : u32 = 0x45564157;
const FMT_ : u32 = 0x20746d66;
const DATA : u32 = 0x61746164;
const LIST : u32 = 0x5453494c;
const FACT : u32 = 0x74636166;

const FORMAT_PCM  : u16 = 1;
const FORMAT_IEEE : u16 = 3;
const FORMAT_EXT  : u16 = 0xfffe;

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Format {
  PCM       = FORMAT_PCM  as isize,
  IEEEFloat = FORMAT_IEEE as isize,
  Extended  = FORMAT_EXT  as isize
}

#[derive(Debug,Copy,Clone)]
pub struct WaveInfo {
  /// Which encoding format this file uses.
  /// If the format is `Format::Extended`, then the actual audio format is
  /// instead determined by the `subformat` field.
  pub audio_format:    Format,
  /// Number of distinct audio channels.
  pub channels:        u16,
  /// Number of audio samples per second.
  pub sample_rate:     u32,
  pub byte_rate:       u32,
  pub block_align:     u16,
  /// Number of bits used to represent each sample.
  pub bits_per_sample: u16,
  /// Number of frames present in the file.  Each frame contains one sample per
  /// channel.
  pub total_frames:    u32,
  pub valid_bps:       Option<u16>,
  pub channel_mask:    Option<u32>,
  /// For `Format::Extended` files, this field contains the actual audo encoding
  /// of the file, either `Format::PCM` or `Format::IEEEFloat`.
  pub subformat:       Option<Format>
}

pub struct WaveFile {
  mmap:        Mmap,
  data_offset: usize,
  data_size:   usize,
  pub info:    WaveInfo
}

pub struct WaveFileIterator<'a> {
  file:             &'a WaveFile,
  pos:              usize,
  base:             usize,
  end:              usize,
  bytes_per_sample: usize,
}

#[derive(Debug,PartialEq)]
pub enum Frame {
  /// Represents a frame from a single-channel file.
  Mono(i32),
  /// Represents a frame from a stereo (2 channel) file.
  Stereo(i32, i32),
  /// Represents a frame from a file with more than two channels.
  Multi(Vec<i32>)
}

impl WaveFile {
  /// Constructs a new `WaveFile`.
  ///
  /// # Example
  ///
  /// ```
  /// use wavefile::{WaveFile,WaveError};
  ///
  /// match WaveFile::open("./fixtures/test-s24le.wav") {
  ///   Ok(f)  => f,
  ///   Err(e) => panic!("Couldn't open example file: {}", e)
  /// };
  /// ```
  pub fn open<S: Into<String>>(path: S) -> Result<WaveFile, WaveError> {
    let filename = path.into();
    let mmap = try!(Mmap::open_path(filename, Protection::Read));
    let info = WaveInfo {
      audio_format:    Format::PCM,
      channels:        0,
      sample_rate:     0,
      byte_rate:       0,
      block_align:     0,
      bits_per_sample: 0,
      total_frames:    0,
      valid_bps:       None,
      channel_mask:    None,
      subformat:       None
    };
    let mut file = WaveFile { mmap: mmap, data_offset: 0, data_size: 0, info: info };

    try!(file.read_header_chunks());

    Ok(file)
  }

  /// Returns an iterator which yields each individual `Frame` successively
  /// until it reaches the end of the file.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use wavefile::WaveFile;
  ///
  /// let wav = WaveFile::open("./fixtures/test-s24le.wav").unwrap();
  ///
  /// for frame in wav.iter() {
  ///   println!("{:?}", frame);
  /// }
  /// ```
  pub fn iter(&self) -> WaveFileIterator {
    let bytes_per_sample = self.info.bits_per_sample as usize / 8;
    WaveFileIterator {
      file:             &self,
      pos:              0,
      base:             self.data_offset,
      end:              self.data_offset + self.data_size,
      bytes_per_sample: bytes_per_sample
    }
  }

  fn read_header_chunks(&mut self) -> Result<(), WaveError> {
    let mut cursor   = Cursor::new(unsafe { self.mmap.as_slice() } );
    let mut have_fmt = false;
    let mut chunk_id = try!(cursor.read_u32::<LittleEndian>());

    let mut chunk_size : u32;

    try!(cursor.read_u32::<LittleEndian>());

    let riff_type = try!(cursor.read_u32::<LittleEndian>());

    if chunk_id != RIFF || riff_type != WAVE {
      return Err(WaveError::ParseError("Not a Wavefile".into()));
    }


    loop {
      chunk_id   = try!(cursor.read_u32::<LittleEndian>());
      chunk_size = try!(cursor.read_u32::<LittleEndian>());

      match chunk_id {
        FMT_ => {
          have_fmt = true;
          self.info.audio_format = match try!(cursor.read_u16::<LittleEndian>()) {
            FORMAT_PCM  => Format::PCM,
            FORMAT_IEEE => Format::IEEEFloat,
            FORMAT_EXT  => Format::Extended,
            other       => {
              let msg = format!("Unexpected format {0:x}", other);
              return Err(WaveError::ParseError(msg));
            }
          };
          self.info.channels        = try!(cursor.read_u16::<LittleEndian>());
          self.info.sample_rate     = try!(cursor.read_u32::<LittleEndian>());
          self.info.byte_rate       = try!(cursor.read_u32::<LittleEndian>());
          self.info.block_align     = try!(cursor.read_u16::<LittleEndian>());
          self.info.bits_per_sample = try!(cursor.read_u16::<LittleEndian>());

          if self.info.audio_format == Format::Extended {
            match try!(cursor.read_u16::<LittleEndian>()) {
              0 => { },
              22 => {
                self.info.valid_bps    = Some(try!(cursor.read_u16::<LittleEndian>()));
                self.info.channel_mask = Some(try!(cursor.read_u32::<LittleEndian>()));
                self.info.subformat    = match try!(cursor.read_u16::<LittleEndian>()) {
                  FORMAT_PCM  => Some(Format::PCM),
                  FORMAT_IEEE => Some(Format::IEEEFloat),
                  other       => {
                    let msg = format!("Unexpected subformat {0:x}", other);
                    return Err(WaveError::ParseError(msg));
                  }
                };
                try!(cursor.seek(SeekFrom::Current(14)));
              },
              x => {
                let msg = format!("Unexpected extension size: {}", x);
                return Err(WaveError::ParseError(msg));
              }
            }

          }
        },
        DATA  => {
          self.data_size = chunk_size as usize;
          break;
        },
        LIST  => { try!(cursor.seek(SeekFrom::Current(chunk_size as i64))); },
        FACT  => { try!(cursor.seek(SeekFrom::Current(chunk_size as i64))); },
        other => {
          let msg = format!("Unexpected Chunk ID {0:x}", other);
          return Err(WaveError::ParseError(msg));
        }
      }
    }

    if !have_fmt {
      return Err(WaveError::ParseError("Format Chunk not found".into()));
    }

    if self.info.audio_format == Format::IEEEFloat {
      return Err(WaveError::Unsupported("IEEE Float format not implemented".into()));
    }
    if self.info.audio_format == Format::Extended && self.info.subformat != Some(Format::PCM) {
      return Err(WaveError::Unsupported("Only PCM data is supported for Ext Wave".into()));
    }

    if self.info.channels == 0 || self.info.bits_per_sample < 8 {
      let msg = format!("Invalid channel count {} or bits per sample {} value",
                        self.info.channels, self.info.bits_per_sample);

      return Err(WaveError::ParseError(msg));
    }

    self.info.total_frames = self.data_size as u32 / (self.info.channels as u32 * self.info.bits_per_sample as u32 / 8 );

    self.data_offset = cursor.position() as usize;
    Ok(())
  }
}

impl<'a> Iterator for WaveFileIterator<'a> {
  type Item = Frame;

  fn next(&mut self) -> Option<Self::Item> {
    let mut cursor = Cursor::new(unsafe { self.file.mmap.as_slice() });
    let info = self.file.info;

    if let Err(_) = cursor.seek(SeekFrom::Start((self.base + self.pos) as u64)) {
      return None;
    };

    if cursor.position() as usize == self.end {
      return None;
    }

    let mut samples : Vec<i32> = Vec::with_capacity(info.channels as usize);

    for _ in 0..info.channels {
      match cursor.read_int::<LittleEndian>(self.bytes_per_sample) {
        Ok(sample) => samples.push(sample as i32),
        Err(e)     => { panic!("{:?}", e); }
      }
    }

    self.pos = cursor.position() as usize - self.base;

    match info.channels {
      0 => unreachable!(),
      1 => Some(Frame::Mono(samples[0])),
      2 => Some(Frame::Stereo(samples[0], samples[1])),
      _ => Some(Frame::Multi(samples))
    }
  }
}

#[test]
fn test_info() {
  let file = match WaveFile::open("./fixtures/test-s24le.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e)
  };

  assert_eq!(file.info.audio_format,    Format::PCM);
  assert_eq!(file.info.channels,        2);
  assert_eq!(file.info.sample_rate,     48000);
  assert_eq!(file.info.byte_rate,       288000);
  assert_eq!(file.info.block_align,     6);
  assert_eq!(file.info.bits_per_sample, 24);
  assert_eq!(file.info.total_frames,    501888);

  let file = match WaveFile::open("./fixtures/test-u8.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e)
  };

  assert_eq!(file.info.audio_format, Format::PCM);
  assert_eq!(file.info.channels,        2);
  assert_eq!(file.info.sample_rate,     48000);
  assert_eq!(file.info.byte_rate,       96000);
  assert_eq!(file.info.bits_per_sample, 8);
  assert_eq!(file.info.block_align,     2);
  assert_eq!(file.info.total_frames,    501888);
}

#[test]
fn test_iter() {
  let file = match WaveFile::open("./fixtures/test-s24le.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e)
  };

  let frames = file.iter().take(2).collect::<Vec<_>>();
  let expected = vec![
    Frame::Stereo(19581, 19581),
    Frame::Stereo(24337, 24337)
  ];

  for i in 0..expected.len() {
    assert_eq!(frames[i], expected[i]);
  }

}


#[test]
fn test_formats() {
  if let Err(e) = WaveFile::open("./fixtures/test-f32le.wav") {
    match e {
      WaveError::Unsupported(_) => true,
      _ => panic!("Unexpected error (expected Unsupported)")
    };
  } else {
    panic!("Unsupported format returned OK?");
  }
}
