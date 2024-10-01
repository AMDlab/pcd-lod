use anyhow::ensure;
use clap::Parser;

use image::DynamicImage;
use pcd_lod::{
    detect_cloudcompare_exists,
    prelude::{Encoder, Meta},
    process_lod, LODUnit,
};

use std::{
    convert::From,
    fs::{canonicalize, create_dir, File},
    io::Write,
};

/// Command line arguments
#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// point cloud file name of the point cloud to be input (.txt, .csv, .las, .xyz, .e57 supported)
    #[clap(short = 'i', long)]
    input_file: String,

    /// folder name to be output
    #[clap(short = 'o', long)]
    output_directory: String,

    /// apply global shift or not (0: no, 1: yes)
    #[clap(long, default_value_t = 0)]
    global_shift: u8,

    /// (Optional) execute path to CloudCompare
    #[clap(long)]
    cloud_compare_path: Option<String>,
}

/// Main handler for CLI
async fn handler() -> anyhow::Result<()> {
    let args: Args = Args::parse();
    let input_file = &args.input_file;
    let output_directory = &args.output_directory;
    let use_global_shift = args.global_shift == 1;
    let exec_path = args.cloud_compare_path.as_ref();

    ensure!(
        detect_cloudcompare_exists(exec_path).is_ok(),
        "CloudCompare is not installed!"
    );

    let output_path = canonicalize(output_directory)?;
    ensure!(output_path.is_dir(), "Output path must be directory");
    let output_path = &output_path;

    let per_unit = |unit: LODUnit| async move {
        let LODUnit {
            lod,
            bounding_box: bbox,
            points: pts,
            x,
            y,
            z,
        } = unit;
        let encoder = Encoder::new(&pts, Some(bbox));
        // let img = encoder.encode_8bit_quad();
        // let img = DynamicImage::from(img);
        // let _ = img.save_with_format(&out_file_path, image::ImageFormat::WebP);
        // let _ = img.save_with_format(out_file_path, image::ImageFormat::Png);

        let mut path = output_path.clone();
        path.push(lod.to_string());
        let _ = create_dir(&path);

        let mut position_image_path = path.clone();
        position_image_path.push(format!("{}-{}-{}.png", x, y, z));
        let mut color_image_path = path.clone();
        color_image_path.push(format!("{}-{}-{}-color.png", x, y, z));
        let (position, color) = encoder.encode_8bit();
        let _ = DynamicImage::from(position)
            .save_with_format(&position_image_path, image::ImageFormat::Png);
        let _ =
            DynamicImage::from(color).save_with_format(&color_image_path, image::ImageFormat::Png);

        Ok(())
    };
    let per_lod = |lod, bounds, coordinates| async move {
        let meta = Meta::new(lod, bounds, coordinates);
        let json = serde_json::to_string(&meta).unwrap();

        let mut meta_file_path = output_path.clone();
        meta_file_path.push("meta.json");
        let mut f = File::create(meta_file_path).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        Ok(())
    };
    process_lod(exec_path, input_file, per_unit, per_lod, use_global_shift).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    match handler().await {
        Ok(_) => {
            println!("success");
        }
        Err(e) => {
            println!("error: {:?}", e);
        }
    }
}
