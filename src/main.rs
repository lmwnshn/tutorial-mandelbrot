extern crate crossbeam;
extern crate image;
extern crate num;

use image::ColorType;
use image::png::PNGEncoder;
use num::Complex;
use std::io::Result;
use std::io::Write;
use std::fs::File;
use std::str::FromStr;

/// escape_time(c, l) : check if `c` in Mandelbrot with up to `l` iterations
///
/// Returns:
///     `Some(i)` if `c` left within `i` iterations, `i` < `l`
///     `None` otherwise
fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }
    None
}

fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T,T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index+1..])) {
                (Ok(l), Ok(r)) => Some ((l,r)),
                _ => None
            }
        }
    }
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

fn pixel_to_point(bounds: (usize, usize),
                  pixel: (usize, usize),
                  top_left: Complex<f64>,
                  bot_right: Complex<f64>)
    -> Complex<f64>
{
    let tl = top_left;
    let br = bot_right;
    let (width, height) = (br.re - tl.re, tl.im - br.im);

    Complex {
        re: tl.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: tl.im - pixel.1 as f64 * height / bounds.1 as f64
    }
}

fn render(pixels: &mut [u8],
          bounds: (usize, usize),
          top_left: Complex<f64>,
          bot_right: Complex<f64>)
{
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0 .. bounds.1 {
        for col in 0 .. bounds.0 {
            let pt = pixel_to_point(bounds, (col, row), top_left, bot_right);

            pixels[row * bounds.0 + col] =
                match escape_time(pt, 255) {
                    None => 0,
                    Some(i) => 255 - i as u8
                }
        }
    }
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize))
    -> Result<()>
{
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels,
                   bounds.0 as u32, bounds.1 as u32,
                   ColorType::Gray(8))?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(std::io::stderr(),
                 "Usage: mandelbrot FILE PIXELS TOP_LEFT BOT_RIGHT")
            .unwrap();
        writeln!(std::io::stderr(),
                "e.g. {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
                args[0])
            .unwrap();
        std::process::exit(1);
    }

    let bounds = parse_pair(&args[2], 'x')
        .expect("error parsing PIXELS");
    let top_left = parse_complex(&args[3])
        .expect("error parsing TOP_LEFT");
    let bot_right = parse_complex(&args[4])
        .expect("error parsing BOT_RIGHT");

    let mut pixels = vec![0; bounds.0 * bounds.1];

    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> =
            pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_top_left =
                    pixel_to_point(bounds, (0, top), top_left, bot_right);
                let band_bot_right =
                    pixel_to_point(bounds, (bounds.0, top+height),
                                   top_left, bot_right);

                spawner.spawn(move || {
                    render(band, band_bounds, band_top_left, band_bot_right);
                });
            }
        })
    }

    write_image(&args[1], &pixels, bounds)
        .expect("error writing PNG file");
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("",','), None);
    assert_eq!(parse_pair::<i32>("10",','), None);
    assert_eq!(parse_pair::<i32>("10,",','), None);
    assert_eq!(parse_pair::<i32>("10,20",','), Some((10,20)));
    assert_eq!(parse_pair::<i32>("10,20x",','), None);
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.25,-0.0625"),
               Some(Complex { re: 1.25, im: -0.0625 }));
    assert_eq!(parse_complex(",1.0"), None)
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100,100), (25,75),
                              Complex { re: -1.0, im:  1.0 },
                              Complex { re:  1.0, im: -1.0 }),
               Complex { re: -0.5, im: -0.5 });
}
