use std::{
    ffi::OsStr,
    fs::{canonicalize, File},
    future::Future,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::ensure;

use point::Point;
use prelude::{BoundingBox, Coordinates, PointCloudMap, PoissonDiskSampling};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

mod bounding_box;
mod color;
mod encoder;
mod meta;
mod point;
mod point_cloud_map;
mod point_cloud_unit;
mod poisson_disk_sampling;

/// key represents level of detail for hash map
type LODKey = (i32, i32, i32);

pub mod prelude {
    pub use crate::bounding_box::*;
    pub use crate::color::*;
    pub use crate::encoder::*;
    pub use crate::meta::*;
    pub use crate::point::*;
    pub use crate::point_cloud_map::*;
    pub use crate::point_cloud_unit::*;
    pub use crate::poisson_disk_sampling::*;
}

/// get Command instance for CloudCompare
/// change the path according to each OS
fn command(path: Option<&String>) -> Command {
    match path {
        Some(path) => Command::new(path),
        None => {
            // https://www.cloudcompare.org/doc/wiki/index.php/Command_line_mode
            #[cfg(target_os = "macos")]
            {
                Command::new("/Applications/CloudCompare.app/Contents/MacOS/CloudCompare")
            }
            #[cfg(target_os = "windows")]
            {
                Command::new("C:\\Program Files\\CloudCompare\\CloudCompare.exe")
            }
            #[cfg(target_os = "linux")]
            Command::new("CloudCompare")
        }
    }
}

/// detect if CloudCompare is installed by executing command
pub fn detect_cloudcompare_exists(path: Option<&String>) -> anyhow::Result<String> {
    let mut cmd = command(path);
    cmd.arg("-SILENT");
    let output = cmd.output()?;
    let msg = std::str::from_utf8(&output.stdout)?;
    Ok(msg.to_string())
}

/// convert pcd file to txt file with CloudCompare
fn convert_pcd_file_to_txt<S0: AsRef<OsStr>, S1: AsRef<OsStr>>(
    cmd: Option<&String>,
    input_file_path: S0,
    out_txt_path: S1,
    drop_global_shift: bool,
) -> anyhow::Result<()> {
    let mut cmd = command(cmd);
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
fn read_points_from_txt(path: &std::path::Path) -> anyhow::Result<Vec<Point>> {
    let f = File::open(path);
    match f {
        Ok(f) => {
            let reader = BufReader::new(f);
            let points = reader
                .lines()
                .map_while(Result::ok)
                .filter_map(|line| Point::try_parse(&line).ok())
                .collect();
            Ok(points)
        }
        _ => Err(anyhow::anyhow!("failed to open file")),
    }
}

/// process level of detail
pub async fn process_lod<F0, F1, Fut0, Fut1>(
    exec_path: Option<&String>,
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

    convert_pcd_file_to_txt(
        exec_path,
        &full_input_file_path,
        &seed_file_path,
        use_global_shift,
    )?;

    println!("Converting pcd to txt is done!");

    // When multiple point clouds are merged and written out with CloudCompare, the suffix of the file name is _0.
    // Therefore, if _0 is attached, use it.
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
    let point_count_threshold = 2_u32.pow(14) as usize; // 16384
                                                        // let point_count_threshold = 2_u32.pow(10) as usize;
    let side = (point_count_threshold as f64).sqrt();

    let mut coordinates = Coordinates::new();

    println!("Start processing...");

    // create root map
    let sampler = PoissonDiskSampling::<f64, Point>::new();
    let size = bounds.size();
    let max_size = size.x.max(size.y).max(size.z);
    let calculate_sampling_radius = |lod: u32| {
        let unit_size = max_size / (lod as f64);
        unit_size / side
    };
    let mut parent_map = {
        let map = PointCloudMap::root(bounds.clone(), &points);
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
                sampler.sample(unit.points(), calculate_sampling_radius(1))
            };
            callback_per_unit(map.bounds().clone(), pts, 0, 0, 0, 0).await?;
        }
        callback_per_lod(map.lod() + 1, bounds.clone(), coordinates.clone()).await?;
        map
    };

    loop {
        let next = parent_map.divide(point_count_threshold);
        let lod = 2_u32.pow(next.lod());
        let sampling_radius = calculate_sampling_radius(lod);

        let has_over_threshold = next
            .map()
            .iter()
            .any(|u| u.1.points.len() >= point_count_threshold);

        let samples = next
            .map()
            .par_iter()
            .map(|(k, u)| {
                let pts = if !has_over_threshold {
                    u.points.clone()
                } else {
                    sampler.sample(u.points(), sampling_radius)
                    // u.points.clone()
                };
                (k, pts)
            })
            .collect::<Vec<_>>();

        for (k, pts) in samples.into_iter() {
            let (x, y, z) = k;
            let c_key = format!("{}-{}-{}", x, y, z);
            let bbox = BoundingBox::from_iter(pts.iter());
            coordinates
                .entry(next.lod())
                .or_default()
                .entry(c_key)
                .or_insert(bbox.clone());
            callback_per_unit(bbox, pts, next.lod(), *x, *y, *z).await?;
        }
        callback_per_lod(next.lod() + 1, bounds.clone(), coordinates.clone()).await?;

        if !has_over_threshold {
            break;
        }

        println!("Processing level:{} is done!", next.lod());

        parent_map = next;
    }

    std::fs::remove_file(&path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn detect_app_exists() {
        let r = super::detect_cloudcompare_exists(None);
        assert!(r.is_ok());
    }
}
