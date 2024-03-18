use anyhow::ensure;
use bounding_box::BoundingBox;
use clap::Parser;

use encoder::Encoder;
use image::DynamicImage;
use meta::{Coordinates, Meta};
use point::Point;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use std::collections::HashMap;
use std::convert::From;
use std::ffi::OsStr;
use std::fs::{canonicalize, create_dir_all, File};
use std::future::Future;
use std::io::{prelude::*, BufReader};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

pub mod bounding_box;
pub mod color;
pub mod encoder;
pub mod lod;
pub mod meta;
pub mod point;
pub mod point_cloud;

pub fn command() -> Command {
    // https://www.cloudcompare.org/doc/wiki/index.php/Command_line_mode
    #[cfg(target_os = "macos")]
    {
        let cmd = Command::new("/Applications/CloudCompare.app/Contents/MacOS/CloudCompare");
        // TODO: or brew install cloudcompare
        cmd
    }
    #[cfg(not(target_os = "macos"))]
    Command::new("CloudCompare")
}

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

type LODKey = (i32, i32, i32);

pub fn detect_cloudcompare_exists() -> anyhow::Result<String> {
    let mut cmd = command();
    cmd.arg("-SILENT");
    let output = cmd.output()?;
    let msg = std::str::from_utf8(&output.stdout)?;
    Ok(msg.to_string())
}

pub fn convert_pcd_file_to_txt<S: AsRef<OsStr>, T: AsRef<OsStr>>(
    input_file_path: S,
    out_txt_path: T,
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

#[allow(unused)]
pub fn subsampling(
    input_file_path: &String,
    out_txt_path: &String,
    spatial: f64,
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
        .arg("-SS")
        .arg("SPATIAL")
        .arg(spatial.to_string())
        .arg("-C_EXPORT_FMT")
        .arg("ASC")
        .arg("-SEP") // separator
        .arg("SPACE");

    if drop_global_shift {
        cmd.arg("-DROP_GLOBAL_SHIFT");
    }

    cmd.arg("-SAVE_CLOUDS").arg("FILE").arg(out_txt_path);

    let output = cmd.output()?;
    let result = output.stdout;
    let msg = std::str::from_utf8(&result)?;
    println!("{}", msg);
    Ok(())
}

pub fn read_points_from_txt(path: &std::path::Path) -> anyhow::Result<Vec<Point>> {
    let f = File::open(path);
    match f {
        Ok(f) => {
            let reader = BufReader::new(f);
            let points = reader
                .lines()
                .flatten()
                .filter_map(|line| Point::parse(&line).ok())
                .collect();
            Ok(points)
        }
        _ => Err(anyhow::anyhow!("failed to open file")),
    }
}

fn generate_points_hash(
    points: &Vec<Point>,
    bounds: &BoundingBox,
    unit: &f64,
) -> HashMap<LODKey, Vec<Point>> {
    let min = &bounds.min;
    let pts: Vec<(LODKey, Point)> = points
        .par_iter()
        .map(|v| {
            let position = v.position;
            let x = position.x;
            let y = position.y;
            let z = position.z;
            let ix = ((x - min.x) / unit).floor() as i32;
            let iy = ((y - min.y) / unit).floor() as i32;
            let iz = ((z - min.z) / unit).floor() as i32;
            let key = (ix, iy, iz);
            (key, v.clone())
        })
        .collect();

    let mut map = HashMap::new();
    for (key, v) in pts {
        map.entry(key).or_insert_with(std::vec::Vec::new).push(v);
    }
    map
}

pub async fn process_lod<F0, F1, Fut0, Fut1>(
    input_file_path: &String,
    callback_per_unit: F0,
    callback_per_lod: F1,
    use_global_shift: bool,
) -> anyhow::Result<()>
where
    F0: Fn(BoundingBox, Vec<Point>, i32, i32, i32, i32) -> Fut0,
    F1: Fn(i32, f64, BoundingBox, Coordinates) -> Fut1,
    Fut0: Future<Output = anyhow::Result<()>>,
    Fut1: Future<Output = anyhow::Result<()>>,
{
    let i_path = PathBuf::from(&input_file_path);

    ensure!(
        i_path.exists(),
        "input file {:?} is not existed!",
        i_path.to_string_lossy()
    );

    let full_input_file_path = canonicalize(&i_path)?;
    dbg!(&full_input_file_path);

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
        "generating seed file is failed!"
    );

    let path = if PathBuf::from(&seed_file_path).exists() {
        seed_file_path
    } else {
        seed_file_path_0
    };

    // get file size of seed_file_path as mega byte
    let file_size = std::fs::metadata(&path)?.len() as f64;
    let mb_size = file_size / 1024. / 1024.;
    let threshold_mb_size = 1.5;

    // get division level from file size
    let level = if mb_size > threshold_mb_size {
        (mb_size / threshold_mb_size).log(8.).ceil().max(1.) as u32
    } else {
        0
    };

    let points = read_points_from_txt(Path::new(&path))?;
    let bounds = BoundingBox::from_iter(points.iter().map(|p| p.position));
    let size = bounds.max_size();
    let level_scale = 2_u32.pow(level);
    let min_unit = size / (level_scale as f64);
    let idivision = level as i32;

    let mut coordinates = Coordinates::new();
    let point_count_threshold = 2_u32.pow(14) as usize;

    println!("start processing... (level: {})", level);

    for lod in 0..=idivision {
        let div = 2_f64.powf(lod as f64);
        let unit = size / div;
        let (cx, cy, cz) = bounds.ceil(unit);

        let mut map = generate_points_hash(&points, &bounds, &unit);
        for x in 0..cx {
            for y in 0..cy {
                for z in 0..cz {
                    let key = (x, y, z);
                    let points = map.remove(&key);
                    if let Some(points) = points {
                        let pts = if points.len() < point_count_threshold {
                            points
                        } else {
                            uniform_sample_points(points, point_count_threshold)
                        };

                        let c_key = format!("{}-{}-{}", x, y, z);
                        let bbox = BoundingBox::from_iter(pts.iter());
                        coordinates
                            .entry(lod)
                            .or_default()
                            .entry(c_key)
                            .or_insert(bbox.clone());
                        callback_per_unit(bbox, pts, lod, x, y, z).await?;
                    }
                }
            }
        }
        callback_per_lod(idivision, min_unit, bounds.clone(), coordinates.clone()).await?;
    }

    std::fs::remove_file(&path)?;

    Ok(())
}

fn uniform_sample_points(points: Vec<Point>, threshold: usize) -> Vec<Point> {
    let n = points.len();
    let u = (n as f64) / (threshold as f64);
    let step = u.ceil() as usize;
    points.into_iter().step_by(step).collect()
}

async fn handler() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    let i_point_cloud = args.input;
    let o_point_cloud = &args.output;
    let use_global_shift = args.global_shift == 1;

    ensure!(
        detect_cloudcompare_exists().is_ok(),
        "CloudCompare is not installed!"
    );

    let mut output_path = PathBuf::from(&o_point_cloud);

    let per_unit = |bbox, pts: Vec<Point>, lod, x, y, z| async move {
        let encoder = Encoder::new(&pts, Some(bbox));
        // let img = encoder.encode_8bit_quad();
        // let img = DynamicImage::from(img);
        // let _ = img.save_with_format(&out_file_path, image::ImageFormat::WebP);
        // let _ = img.save_with_format(out_file_path, image::ImageFormat::Png);

        let prefix = format!("{}/{}/{}-{}-{}", o_point_cloud, lod, x, y, z);
        let (position, color) = encoder.encode_8bit();
        let _ = DynamicImage::from(position)
            .save_with_format(format!("{}.png", &prefix,), image::ImageFormat::Png);
        let _ = DynamicImage::from(color)
            .save_with_format(format!("{}-color.png", &prefix), image::ImageFormat::Png);

        Ok(())
    };
    let per_lod = |lod, unit, bounds, coordinates| async move {
        let meta = Meta {
            lod,
            bounds,
            coordinates,
        };
        let json = serde_json::to_string(&meta).unwrap();
        let out_file_path = format!("{}/meta.json", o_point_cloud);
        let mut f = std::fs::File::create(out_file_path).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        Ok(())
    };
    let _ = process_lod(&i_point_cloud, per_unit, per_lod, use_global_shift).await?;

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
    use std::{
        fs::{self},
        io::Write,
        path::PathBuf,
    };

    use image::DynamicImage;
    use serial_test::serial;

    use crate::{convert_pcd_file_to_txt, encoder::Encoder, meta::Meta, point::Point, process_lod};

    fn get_data_dir() -> String {
        std::fs::canonicalize(PathBuf::from("./data"))
            .unwrap()
            .as_path()
            .display()
            .to_string()
    }

    async fn pclod_test(name: &str) {
        let dir = get_data_dir();
        let in_file_path = format!("{}/{}", &dir, name);
        let per_unit = |_, _txt, _lod, _x, _y, _z| async move {
            // dbg!(format!("{} ({}, {}, {})", &lod, &x, &y, &z));
            Ok(())
        };
        let per_lod = |_idivs, _min_unit, _bounds, _coordinates| async move {
            // dbg!("per lod");
            Ok(())
        };
        let _ = process_lod(&in_file_path, per_unit, per_lod, false).await;
    }

    #[allow(unused)]
    async fn pclod_test_with_save(name: &str) {
        let dir = get_data_dir();
        let root = format!("{}/processed", &dir);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::create_dir(&root);
        let in_file_path = format!("{}/{}", &dir, name);
        let per_unit = |bbox, pts: Vec<Point>, lod, x, y, z| async move {
            let name = format!("{}-{}-{}-{}.txt", &lod, &x, &y, &z);
            let out_file_path = format!("{}/processed/{}", get_data_dir(), name);

            // let mut hdl = File::create(&out_file_path).unwrap();
            // let txt = convert_points_to_txt(&pts);
            // hdl.write_all(txt.as_bytes()).unwrap();

            let name = format!("{}-{}-{}-{}.png", &lod, &x, &y, &z);
            let out_file_path = format!("{}/processed/{}", get_data_dir(), name);
            let encoder = Encoder::new(&pts, Some(bbox));
            let img = encoder.encode_8bit_quad();
            let img = DynamicImage::from(img);
            // let _ = img.save_with_format(&out_file_path, image::ImageFormat::WebP);
            let _ = img.save_with_format(out_file_path, image::ImageFormat::Png);

            /*
            let encoder = Encoder::new(&pts, Some(bbox));
            let (position, color) = encoder.encode_8bit();
            let _ = DynamicImage::from(position).save_with_format(
                format!(
                    "{}/processed/{}",
                    get_data_dir(),
                    format!("{}-{}-{}-{}.webp", &lod, &x, &y, &z)
                ),
                // image::ImageFormat::Png,
                image::ImageFormat::WebP,
            );
            let _ = DynamicImage::from(color).save_with_format(
                format!(
                    "{}/processed/{}",
                    get_data_dir(),
                    format!("{}-{}-{}-{}-color.webp", &lod, &x, &y, &z)
                ),
                image::ImageFormat::WebP,
            );
            */

            Ok(())
        };
        let per_lod = |lod, unit, bounds, coordinates| async move {
            let meta = Meta {
                lod,
                bounds,
                coordinates,
            };
            let json = serde_json::to_string(&meta).unwrap();
            let out_file_path = format!("{}/processed/meta.json", get_data_dir());
            let mut f = std::fs::File::create(out_file_path).unwrap();
            f.write_all(json.as_bytes()).unwrap();

            Ok(())
        };
        process_lod(&in_file_path, per_unit, per_lod, false).await;
    }

    #[tokio::test]
    #[serial]
    async fn txt_format_test() {
        pclod_test("pcd.txt").await;
    }

    #[tokio::test]
    #[serial]
    async fn csv_format_test() {
        pclod_test("BLNo75-76.csv").await;
    }

    #[tokio::test]
    #[serial]
    async fn xyz_format_test() {
        pclod_test("BLNo74-75.xyz").await;
    }

    #[tokio::test]
    #[serial]
    async fn las_format_test() {
        pclod_test_with_save("Scaniverse.las").await;
    }

    #[tokio::test]
    #[serial]
    async fn bunny_format_csv() {
        pclod_test("bunny.csv").await;
    }

    #[tokio::test]
    #[serial]
    async fn large_txt_test() {
        pclod_test_with_save("uav.txt").await;
        // pclod_test_with_save("D5.txt").await;
    }

    #[tokio::test]
    #[serial]
    async fn e57_format_test() {
        pclod_test_with_save("bunnyDouble.e57").await;
    }

    #[tokio::test]
    #[serial]
    async fn las_to_txt_test() {
        let dir = get_data_dir();
        let name = "PointCloud.laz";
        let input_file_path = format!("{}/{}", &dir, name);
        let mut opath = PathBuf::from(&input_file_path);
        opath.set_extension("txt");
        let out_file_path = String::from(opath.to_str().unwrap());
        let _ = convert_pcd_file_to_txt(&input_file_path, &out_file_path, false);
        /*
        let min_sampling_distance = 1. / 128.;
        let _ = subsampling(
            &input_file_path,
            &out_file_path,
            min_sampling_distance,
            false,
        );
        */
    }

    #[tokio::test]
    #[serial]
    async fn level_test() {
        pclod_test_with_save("uav_20201112_south-0-0-1.txt").await;
        // pclod_test_with_save("Scaniverse.txt").await;
        // pclod_test_with_save("bunny.csv").await;
        // pclod_test_with_save("BLNo74-75.xyz").await;
    }

    #[test]
    fn detect_app_exists() {
        let r = super::detect_cloudcompare_exists();
        assert!(r.is_ok());
    }
}
