use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    input: PathBuf,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    println!("{:?}", args);

    let img = image::open(&args.input).unwrap();
    let output = args.output.unwrap_or_else(|| {
        let mut p = args.input.clone();
        p.set_extension("bmp");
        p
    });

    let img = img.to_rgba8();
    std::fs::write(output, img.as_raw())?;

    // let dds = image_dds::dds_from_image(
    //     &img,
    //     image_dds::ImageFormat::BC3RgbaUnorm, // DXT5圧縮フォーマット
    //     image_dds::Quality::Normal,
    //     image_dds::Mipmaps::Disabled,
    // )?;
    // println!("export {output:?}");
    // let mut file = std::fs::File::create(output)?;
    // dds.write(&mut file)?;

    Ok(())
}
