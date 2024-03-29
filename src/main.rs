use clap::Parser;

use pcd_lod::generate_lod;

use std::convert::From;

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

///
async fn handler() -> anyhow::Result<()> {
    let args: Args = Args::parse();
    let input_file = &args.input_file;
    let output_directory = &args.output_directory;
    let use_global_shift = args.global_shift == 1;
    let exec_path = args.cloud_compare_path.as_ref();
    generate_lod(input_file, output_directory, use_global_shift, exec_path).await
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
