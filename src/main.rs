use anyhow::ensure;
use bounding_box::BoundingBox;
use clap::Parser;

use encoder::Encoder;
use image::DynamicImage;
use meta::{Coordinates, Meta};
use point::Point;
use point_cloud_map::*;

use rayon::prelude::ParallelIterator;

use std::convert::From;
use std::ffi::OsStr;
use std::fs::{canonicalize, create_dir, File};
use std::future::Future;
use std::io::{prelude::*, BufReader};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod bounding_box;
pub mod color;
pub mod encoder;
pub mod meta;
pub mod point;
pub mod point_cloud_map;
pub mod point_cloud_unit;

/// get Command instance for CloudCompare
/// change the path according to each OS
pub fn command() -> Command {
    // https://www.cloudcompare.org/doc/wiki/index.php/Command_line_mode
    #[cfg(target_os = "macos")]
    {
        // TODO: or brew install cloudcompare
        Command::new("/Applications/CloudCompare.app/Contents/MacOS/CloudCompare")
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("C:\\Program Files\\CloudCompare\\CloudCompare.exe")
    }
    #[cfg(target_os = "linux")]
    Command::new("CloudCompare")
}

/// Command line arguments
#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    /// point cloud file name of the point cloud to be input (.txt, .csv, .las, .xyz, .e57 supported)
    #[clap(long)]
    input: String,

    /// folder name to be output
    #[clap(long)]
    output: String,

    /// apply global shift
    #[clap(long, default_value_t = 0)]
    global_shift: u8,
}

/// detect if CloudCompare is installed by executing command
pub fn detect_cloudcompare_exists() -> anyhow::Result<String> {
    let mut cmd = command();
    cmd.arg("-SILENT");
    let output = cmd.output()?;
    let msg = std::str::from_utf8(&output.stdout)?;
    Ok(msg.to_string())
}

/// convert pcd file to txt file with CloudCompare
pub fn convert_pcd_file_to_txt<S0: AsRef<OsStr>, S1: AsRef<OsStr>>(
    input_file_path: S0,
    out_txt_path: S1,
    drop_global_shift: bool,
) -> anyhow::Result<()> {
    let mut cmd = command();
    cmd.arg("-SILENT")
        .arg("-AUTO_SAVE")
        .arg("OFF")
        .arg("-O")
        // CAUTION!: global shift fixes accuracy errors
        // [ccGlobalShiftManager] Entity has very big coordinates: original accuracy may be lost! (you should apply a Global Shift or Scale)
        .arg("-GLOBAL_SHIFT")
        .arg("AUTO")
        .arg(input_file_path)
        .arg("-C_EXPORT_FMT")
        .arg("ASC")
        .arg("-SEP") // separator
        .arg("SPACE");

    if drop_global_shift {
        cmd.arg("-DROP_GLOBAL_SHIFT");
    }

    cmd.arg("-MERGE_CLOUDS");
    cmd.arg("-SAVE_CLOUDS").arg("FILE").arg(out_txt_path);

    let output = cmd.output()?;
    let msg = std::str::from_utf8(&output.stdout)?;
    println!("{}", msg);
    Ok(())
}

/// read points from txt file
pub fn read_points_from_txt(path: &std::path::Path) -> anyhow::Result<Vec<Point>> {
    let f = File::open(path);
    match f {
        Ok(f) => {
            let reader = BufReader::new(f);
            let points = reader
                .lines()
                .map_while(Result::ok)
                .filter_map(|line| Point::parse(&line).ok())
                .collect();
            Ok(points)
        }
        _ => Err(anyhow::anyhow!("failed to open file")),
    }
}

/// key represents level of detail for hash map
type LODKey = (i32, i32, i32);

/// process level of detail
pub async fn process_lod<F0, F1, Fut0, Fut1>(
    input_file_path: &String,
    callback_per_unit: F0,
    callback_per_lod: F1,
    use_global_shift: bool,
) -> anyhow::Result<()>
where
    F0: Fn(BoundingBox, Vec<Point>, u32, i32, i32, i32) -> Fut0,
    F1: Fn(u32, BoundingBox, Coordinates) -> Fut1,
    Fut0: Future<Output = anyhow::Result<()>>,
    Fut1: Future<Output = anyhow::Result<()>>,
{
    let i_path = PathBuf::from(&input_file_path);

    ensure!(
        i_path.exists(),
        "Input file {:?} is not existed!",
        i_path.to_string_lossy()
    );

    let full_input_file_path = canonicalize(&i_path)?;

    let mut o_path = full_input_file_path.clone();

    // Create initial pcd with txt format
    o_path.set_file_name("seed.txt");

    let seed_file_path = String::from(o_path.to_str().unwrap());

    println!("Converting pcd to txt...");

    convert_pcd_file_to_txt(&full_input_file_path, &seed_file_path, use_global_shift)?;

    println!("Converting pcd to txt is done!");

    // CAUTION:
    //  CloudCompareで複数の点群をmergeした上で書き出すと、ファイル名のsuffixに_0が付与される
    o_path.set_file_name("seed.txt_0");
    let seed_file_path_0 = String::from(o_path.to_str().unwrap());

    ensure!(
        PathBuf::from(&seed_file_path).exists() || PathBuf::from(&seed_file_path_0).exists(),
        "Generating seed file is failed!"
    );

    let path = if PathBuf::from(&seed_file_path).exists() {
        seed_file_path
    } else {
        seed_file_path_0
    };

    let points = read_points_from_txt(Path::new(&path))?;
    let bounds = BoundingBox::from_iter(points.iter().map(|p| p.position));
    let point_count_threshold = 2_u32.pow(14) as usize;

    let mut coordinates = Coordinates::new();

    println!("Start processing...");

    // create root map
    let mut parent_map = {
        let map = PointCloudMap::root(0, bounds.clone(), &points);
        let points = map.map().get(&(0, 0, 0));
        if let Some(unit) = points {
            let c_key = format!("{}-{}-{}", 0, 0, 0);
            coordinates
                .entry(map.lod())
                .or_default()
                .entry(c_key)
                .or_insert(map.bounds().clone());
            let under_threshold = unit.points.len() < point_count_threshold;
            let pts = if under_threshold {
                unit.points.clone()
            } else {
                unit.uniform_sample_points(point_count_threshold)
            };
            callback_per_unit(map.bounds().clone(), pts, 0, 0, 0, 0).await?;
        }
        callback_per_lod(map.lod() + 1, bounds.clone(), coordinates.clone()).await?;
        map
    };

    loop {
        let next = parent_map.divide(point_count_threshold);

        let mut all_under_threshold = true;

        for (k, v) in next.map().iter() {
            let (x, y, z) = k;
            let c_key = format!("{}-{}-{}", x, y, z);
            let under_threshold = v.points.len() < point_count_threshold;
            let pts = if under_threshold {
                v.points.clone()
            } else {
                v.uniform_sample_points(point_count_threshold)
            };
            let bbox = BoundingBox::from_iter(pts.iter());
            coordinates
                .entry(next.lod())
                .or_default()
                .entry(c_key)
                .or_insert(bbox.clone());
            callback_per_unit(bbox, pts, next.lod(), *x, *y, *z).await?;

            all_under_threshold = all_under_threshold && under_threshold;
        }
        callback_per_lod(next.lod() + 1, bounds.clone(), coordinates.clone()).await?;

        if all_under_threshold {
            break;
        }

        println!("Processing level:{} is done!", next.lod());

        parent_map = next;
    }

    std::fs::remove_file(&path)?;

    Ok(())
}

///
async fn handler() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    let i_point_cloud = args.input;
    let o_point_cloud = &args.output;
    let use_global_shift = args.global_shift == 1;

    ensure!(
        detect_cloudcompare_exists().is_ok(),
        "CloudCompare is not installed!"
    );

    let output_path = canonicalize(o_point_cloud)?;
    ensure!(output_path.is_dir(), "Output path must be directory");
    let output_path = &output_path;

    let per_unit = |bbox, pts: Vec<Point>, lod: u32, x: i32, y: i32, z: i32| async move {
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
        let meta = Meta {
            lod,
            bounds,
            coordinates,
        };
        let json = serde_json::to_string(&meta).unwrap();

        let mut meta_file_path = output_path.clone();
        meta_file_path.push("meta.json");
        let mut f = File::create(meta_file_path).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        Ok(())
    };
    process_lod(&i_point_cloud, per_unit, per_lod, use_global_shift).await?;

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

#[cfg(test)]
mod tests {
    #[test]
    fn detect_app_exists() {
        let r = super::detect_cloudcompare_exists();
        assert!(r.is_ok());
    }
}
