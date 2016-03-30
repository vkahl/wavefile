// This example will generate a basic waveform image given a wavefile.
// Usage:  `cargo run -- /path/to/foo.wav output.png --width 400 --height 300`

extern crate argparse;
extern crate wavefile;
extern crate image;
extern crate itertools;

use argparse::{ArgumentParser, Store};
use wavefile::WaveFile;
use itertools::Itertools;
use image::{GenericImage,ImageBuffer,Rgba,Pixel};

struct Arguments {
  input:      String,
  output:     String,
  dimensions: (u32, u32)
}

fn main() {
  // default arguments.
  let mut args = Arguments {
    input:      "".into(),
    output:     "".into(),
    dimensions: (400, 300)
  };

  {
    let mut ap = ArgumentParser::new();

    ap.set_description("Generate a waveform image from a wave file.");

    ap.refer(&mut args.input)
      .add_argument("input file", Store, "WAV file to read.")
      .required();
    ap.refer(&mut args.output)
      .add_argument("output file", Store, "output PNG file to write.")
      .required();
    ap.refer(&mut args.dimensions.0)
      .add_option(&["-w", "--width"], Store, "output image width");
    ap.refer(&mut args.dimensions.1)
      .add_option(&["-h", "--height"], Store, "output image height");

    ap.parse_args_or_exit();
  }

  // open the wave file.
  let wav = match WaveFile::open(args.input) {
    Ok(f)  => f,
    Err(e) => panic!("{}",  e)
  };

  // set up the output image with the specified dimensions.
  let mut png = ImageBuffer::<Rgba<u8>, Vec<_>>::new(
    args.dimensions.0,
    args.dimensions.1
  );

  // we want to divide the frames in the wavefile into chunks,
  // so that we have one chunk per horizontal pixel in the output image.
  let chunk_size = wav.len() / args.dimensions.0 as usize;
  let chunks = &wav.iter().chunks_lazy(chunk_size);

  // here we compute the height of the waveform for each chunk, using the max
  // absolute value found in the chunk of frames (from any channel).
  // we could also use the average value, perhaps.
  let values = chunks.into_iter().map( |chunk| {
    let max = chunk.into_iter().map( |frame| {
      frame.iter().map(|sample| sample.abs()).max().unwrap()
    }).max().unwrap();
    max
  }).take(args.dimensions.0 as usize).collect::<Vec<i32>>();

  // we'll scale everything by the absolute max value,
  // so that one point will touch the edge of the image.
  let global_max = *values.iter().max().unwrap();
  let mid        = args.dimensions.1 / 2;
  let scale      = mid as f32 / global_max as f32;
  // a beautiful solid red.
  let color      = Rgba::from_channels(255u8, 0u8, 0u8, 255u8);

  for (x, value) in values.iter().enumerate() {
    // calculate the height above the midpoint for this x value.
    let height = (*value as f32 * scale) as u32;
    // draw the line mirrored across the mid point.
    for y in (mid - height)..(mid + height) {
      png.put_pixel(x as u32, y, color);
    }
  }

  // tada.
  match png.save(&args.output) {
    Ok(_)  => println!("Wrote {}", &args.output),
    Err(e) => println!("Couldn't write {}: {}", &args.output, e)
  };
}
