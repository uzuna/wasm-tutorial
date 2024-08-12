use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use image::DynamicImage;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
#[clap(rename_all = "kebab-case")]
enum Format {
    // フォントなど明度のみを持つ画像
    Luminance,
    #[default]
    Bitmap,
    Dxt1,
    Dxt3,
    Dxt5,
}

impl Format {
    fn output_extension(&self) -> &'static str {
        match self {
            Format::Luminance => "lum",
            Format::Bitmap => "bmp",
            Format::Dxt1 => "dxt1",
            Format::Dxt3 => "dxt3",
            Format::Dxt5 => "dxt5",
        }
    }

    fn encode(&self, img: &DynamicImage) -> anyhow::Result<Vec<u8>> {
        match self {
            Format::Luminance => {
                let img = img.to_luma8();
                Ok(img.into_raw().to_vec())
            }
            Format::Bitmap => {
                let img = img.to_rgba8();
                Ok(img.into_raw().to_vec())
            }
            Format::Dxt1 => self.encode_dds(img, image_dds::ImageFormat::BC1RgbaUnorm),
            Format::Dxt3 => self.encode_dds(img, image_dds::ImageFormat::BC2RgbaUnorm),
            Format::Dxt5 => self.encode_dds(img, image_dds::ImageFormat::BC3RgbaUnorm),
        }
    }

    fn encode_dds(
        &self,
        img: &DynamicImage,
        format: image_dds::ImageFormat,
    ) -> anyhow::Result<Vec<u8>> {
        let img = img.to_rgba8();
        let dds = image_dds::dds_from_image(
            &img,
            format,
            image_dds::Quality::Normal,
            image_dds::Mipmaps::Disabled,
        )?;
        let mut buf = Vec::new();
        dds.write(&mut buf)?;
        Ok(buf)
    }
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    input: PathBuf,
    #[clap(short, long)]
    output: Option<PathBuf>,

    #[clap(
        short,
        long,
        value_enum,
        default_value = "bitmap",
        value_delimiter = ','
    )]
    format: Vec<Format>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    println!("{:?}", args);

    let img = image::open(&args.input).unwrap();

    for f in &args.format {
        let output = match args.output {
            Some(ref p) => p.clone(),
            None => {
                let mut p = args.input.clone();
                p.set_extension(f.output_extension());
                p
            }
        };

        let buf = f.encode(&img)?;
        println!("export {output:?}: {} bytes", buf.len());
        std::fs::write(output, buf)?;
    }

    Ok(())
}
