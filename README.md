# pcd-lod

A generator project for converting point cloud files into a Level of Details (LOD) format.

The entered point cloud files `(.txt, .csv, .las, .xyz, .e57)` are subdivided according to the structure of an octree, until a certain density is reached.

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

## Specification

The LOD subdivided point clouds are normalized using the bounding box in each unit of the octree so that the xyz coordinates fit within a range of 0.0 to 1.0.
Consequently, they are saved as PNG images.
To restore the original xyz values from the PNG images, it is necessary to use the bounding box information of each octree unit, which is included in the meta.json stored in the output folder.

The files outputted in the specified folder by the pcd-lod generator include:

- `meta.json` (the number of LOD subdivisions and the bounding box information of the point clouds contained in each unit of the octree)
- PNG files indicating the positions of point clouds in each unit of the octree _(e.g., `1/0-3-1.png` where the folder name indicates the level of division, and the numbers in the file name represent the octree's address)_
- PNG files indicating the colors of point clouds in each unit of the octree _(e.g., `1/0-3-1-color.png` where the folder name indicates the level of division, and the numbers in the file name represent the octree's address)_
