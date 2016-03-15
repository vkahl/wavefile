#![feature(question_mark)]
extern crate memmap;
extern crate byteorder;

use std::io::{self,Seek,SeekFrom,Cursor};
use std::fmt::{self,Display};
use std::error::{self};
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

#[derive(Debug)]
pub enum WaveError {
  IoError(io::Error),
  Unsupported(String),
  ParseError(String)
}

#[derive(Debug,Copy,Clone)]
pub struct WaveInfo {
  pub audio_format:    Format,
  pub channels:        u16,
  pub sample_rate:     u32,
  pub byte_rate:       u32,
  pub block_align:     u16,
  pub bits_per_sample: u16,
  pub total_frames:    u32,
  pub valid_bps:       Option<u16>,
  pub channel_mask:    Option<u32>,
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
  Mono(i32),
  Stereo(i32, i32),
  Multi(Vec<i32>)
}

impl From<io::Error> for WaveError {
  fn from(e: io::Error) -> Self {
    WaveError::IoError(e)
  }
}

impl From<byteorder::Error> for WaveError {
  fn from(e: byteorder::Error) -> Self {
    match e {
      byteorder::Error::UnexpectedEOF => WaveError::ParseError("Unexpected EOF".into()),
      byteorder::Error::Io(e) => WaveError::IoError(e)
    }
  }
}

impl WaveFile {
  pub fn open<S: Into<String>>(path: S) -> Result<WaveFile, WaveError> {
    let filename = path.into();
    let mmap = Mmap::open_path(filename, Protection::Read)?;
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

    file.read_header_chunks()?;

    Ok(file)
  }

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
    let mut chunk_id = cursor.read_u32::<LittleEndian>()?;

    let mut chunk_size : u32;

    cursor.read_u32::<LittleEndian>()?;

    let riff_type = cursor.read_u32::<LittleEndian>()?;

    if chunk_id != RIFF || riff_type != WAVE {
      return Err(WaveError::ParseError("Not a Wavefile".into()));
    }


    loop {
      chunk_id   = cursor.read_u32::<LittleEndian>()?;
      chunk_size = cursor.read_u32::<LittleEndian>()?;

      match chunk_id {
        FMT_ => {
          have_fmt = true;
          self.info.audio_format = match cursor.read_u16::<LittleEndian>()? {
            FORMAT_PCM  => Format::PCM,
            FORMAT_IEEE => Format::IEEEFloat,
            FORMAT_EXT  => Format::Extended,
            other       => {
              let msg = format!("Unexpected format {0:x}", other);
              return Err(WaveError::ParseError(msg));
            }
          };
          self.info.channels        = cursor.read_u16::<LittleEndian>()?;
          self.info.sample_rate     = cursor.read_u32::<LittleEndian>()?;
          self.info.byte_rate       = cursor.read_u32::<LittleEndian>()?;
          self.info.block_align     = cursor.read_u16::<LittleEndian>()?;
          self.info.bits_per_sample = cursor.read_u16::<LittleEndian>()?;

          if self.info.audio_format == Format::Extended {
            match cursor.read_u16::<LittleEndian>()? {
              0 => { },
              22 => {
                self.info.valid_bps    = Some(cursor.read_u16::<LittleEndian>()?);
                self.info.channel_mask = Some(cursor.read_u32::<LittleEndian>()?);
                self.info.subformat    = match cursor.read_u16::<LittleEndian>()? {
                  FORMAT_PCM  => Some(Format::PCM),
                  FORMAT_IEEE => Some(Format::IEEEFloat),
                  other       => {
                    let msg = format!("Unexpected subformat {0:x}", other);
                    return Err(WaveError::ParseError(msg));
                  }
                };
                cursor.seek(SeekFrom::Current(14))?;
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
        LIST  => { cursor.seek(SeekFrom::Current(chunk_size as i64))?; },
        FACT  => { cursor.seek(SeekFrom::Current(chunk_size as i64))?; },
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

impl error::Error for WaveError {
  fn description(&self) -> &str {
    match self {
      &WaveError::ParseError(ref s)  => &s,
      &WaveError::Unsupported(ref s) => &s,
      &WaveError::IoError(ref e)     => e.description()
    }
  }

  fn cause(&self) -> Option<&error::Error> {
    match self {
      &WaveError::IoError(ref e) => Some(e),
      _ => None
    }
  }
}

impl Display for WaveError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &WaveError::IoError(ref e)     => write!(f, "IO Error: {}", e),
      &WaveError::ParseError(ref s)  => write!(f, "Parse Error: {}", s),
      &WaveError::Unsupported(ref s) => write!(f, "Unsupported Format Error: {}", s)
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
