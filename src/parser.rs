use std::str;
use nom::{HexDisplay,Needed,IResult,le_u8,le_u16,le_u32,length_value};
use nom::Err;
use nom::IResult::*;
use memmap::{Mmap,Protection};

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

named!(riff_chunk<(&str, u32)>, pair!(map_res!(tag!("RIFF"), str::from_utf8), le_u32));
named!(format_chunk<(&str, u32)>, pair!(map_res!(tag!("FMT "), str::from_utf8), le_u32));

named!(wave_info<WaveInfo>, chain!(
    riff_chunk ~
    tag!("WAVE") ~
    format_chunk ~
    audio_format:    le_u16 ~
    channels:        le_u16 ~
    sample_rate:     le_u32 ~
    byte_rate:       le_u32 ~
    block_align:     le_u16 ~
    bits_per_sample: le_u16,
  || {
    WaveInfo{
      audio_format:    match audio_format {
        FORMAT_PCM  => Format::PCM,
        FORMAT_IEEE => Format::IEEEFloat,
        FORMAT_EXT  => Format::Extended,
        f => unreachable!(format!("Bad format {:?}", f))
      },
      channels:        channels,
      sample_rate:     sample_rate,
      byte_rate:       byte_rate,
      block_align:     block_align,
      bits_per_sample: bits_per_sample,
      total_frames:    0,
      valid_bps:       None,
      channel_mask:    None,
      subformat:       None
    }
  }
));
fn parse_wav(bytes: &[u8]) {
  
}

#[test]
fn test_things() {
  let mmap = Mmap::open_path("./fixtures/test-s24le.wav", Protection::Read).unwrap();
  let data = unsafe { mmap.as_slice() };

  if let Done(bytes, info) = wave_info(data) {
    println!("{:?}", info);
  }
}

