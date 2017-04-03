use std::io;
use std::fmt::{self,Display};
use std::error::Error;

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

impl Error for WaveError {
  fn description(&self) -> &str {
    match *self {
      WaveError::ParseError(ref s)  |
      WaveError::Unsupported(ref s) => &s,
      WaveError::IoError(ref e)     => e.description()
    }
  }

  fn cause(&self) -> Option<&Error> {
    if let WaveError::IoError(ref e) = *self { Some(e) } else { None }
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
