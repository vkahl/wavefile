use std::io;
use std::fmt::{self,Display};
use std::error::Error;

use byteorder;

#[derive(Debug)]
pub enum WaveError {
  IoError(io::Error),
  Unsupported(String),
  ParseError(String)
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

impl Error for WaveError {
  fn description(&self) -> &str {
    match self {
      &WaveError::ParseError(ref s)  => &s,
      &WaveError::Unsupported(ref s) => &s,
      &WaveError::IoError(ref e)     => e.description()
    }
  }

  fn cause(&self) -> Option<&Error> {
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
