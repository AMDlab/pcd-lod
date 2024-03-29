# pcd-lod

LOD generator for PCD project.

## Usage

```bash
Usage: pcd-lod [OPTIONS] --input-file <INPUT_FILE> --output-directory <OUTPUT_DIRECTORY>

Options:
  -i, --input-file <INPUT_FILE>
          point cloud file name of the point cloud to be input (.txt, .csv, .las, .xyz, .e57 supported)
  -o, --output-directory <OUTPUT_DIRECTORY>
          folder name to be output
      --global-shift <GLOBAL_SHIFT>
          apply global shift or not (0: no, 1: yes) [default: 0]
      --cloud-compare-path <CLOUD_COMPARE_PATH>
          (Optional) execute path to CloudCompare
  -h, --help
          Print help
  -V, --version
          Print version
```
