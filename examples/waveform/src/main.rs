// This example will generate a basic waveform image given a wavefile.
// Usage:  `cargo run -- /path/to/foo.wav output.png --width 400 --height 300`


extern crate argparse;
use argparse::{ ArgumentParser, Store };

extern crate wavefile;
use wavefile::WaveFile;

extern crate image;
use image::{ ImageBuffer,Rgba,Pixel };

extern crate itertools;
use itertools::Itertools;


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
  let chunks = &wav.iter().chunks(chunk_size);

  // here we compute the lowest and highest point of the waveform for each
  // chunk, using the min and max values found in the chunk of frames
  // (from any channel).
  let values = chunks.into_iter().map(|chunk| {
    let mut min = 0.0;
    let mut max = 0.0;

    for frame in chunk {
      for sample in frame {
        if sample > max {
          max = sample;
        } else if sample < min {
          min = sample;
        }
      }
    }

    (min, max)
  }).take(args.dimensions.0 as usize).collect::<Vec<(f32, f32)>>();

  // we'll scale everything by half the image height,
  // so that the edges of the image represent a level of 0dBFS.
  let mid        = args.dimensions.1 / 2;
  let scale      = args.dimensions.1 as f32 / 2.0;

  // a beautiful solid red.
  let color      = Rgba::from_channels(255u8, 0u8, 0u8, 255u8);

  for (x, value) in values.iter().enumerate() {
    // calculate the heights above and below the midpoint for this x value.
    let below = (-value.0 * scale) as u32;
    let above = (value.1 * scale) as u32;

    // draw the line
    for y in (mid - below)..(mid + above) {
      png.put_pixel(x as u32, y, color);
    }
  }

  // tada.
  match png.save(&args.output) {
    Ok(_)  => println!("Wrote {}", &args.output),
    Err(e) => println!("Couldn't write {}: {}", &args.output, e)
  };
}
