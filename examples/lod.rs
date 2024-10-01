use std::{
    fs::{self, File},
    io::BufReader,
};

use bevy::{
    prelude::*,
    render::{
        camera::ScalingMode,
        mesh::{PrimitiveTopology, VertexAttributeValues},
    },
};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_points::{
    material::PointsShaderSettings, mesh::PointsMesh, plugin::PointsPlugin, prelude::PointsMaterial,
};
use bevy_polyline::{
    prelude::{Polyline, PolylineBundle, PolylineMaterial},
    PolylinePlugin,
};
use image::{codecs::png, ImageDecoder};
use itertools::Itertools;
use nalgebra::{coordinates, Point2, Point3, Vector2, Vector3};
use pcd_lod::prelude::{BoundingBox, Meta, Point, PoissonDiskSampling};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InfiniteGridPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(PointsPlugin)
        .add_plugins(PolylinePlugin)
        .add_plugins(AppPlugin)
        .run();
}
struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, close_on_esc);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PointsMaterial>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = format!("{}/examples/output", root);
    let meta: Meta =
        serde_json::from_str(&fs::read_to_string(format!("{}/meta.json", dir)).unwrap()).unwrap();

    let lod = meta.lod();
    let bounds = meta.bounds();
    let center: Vec3 = bounds.center().cast::<f32>().into();
    let transform = Transform::from_translation(-center);

    let spawn_bounding_box = |commands: &mut Commands,
                              polylines: &mut ResMut<'_, Assets<Polyline>>,
                              polyline_materials: &mut ResMut<'_, Assets<PolylineMaterial>>,
                              bounds: BoundingBox,
                              transform: Transform| {
        let p00 = Point3::new(bounds.min.x, bounds.min.y, bounds.min.z);
        let p01 = Point3::new(bounds.max.x, bounds.min.y, bounds.min.z);
        let p02 = Point3::new(bounds.max.x, bounds.max.y, bounds.min.z);
        let p03 = Point3::new(bounds.min.x, bounds.max.y, bounds.min.z);
        let p10 = Point3::new(bounds.min.x, bounds.min.y, bounds.max.z);
        let p11 = Point3::new(bounds.max.x, bounds.min.y, bounds.max.z);
        let p12 = Point3::new(bounds.max.x, bounds.max.y, bounds.max.z);
        let p13 = Point3::new(bounds.min.x, bounds.max.y, bounds.max.z);
        let lines = vec![
            (p00, p01),
            (p01, p02),
            (p02, p03),
            (p03, p00),
            (p00, p10),
            (p01, p11),
            (p02, p12),
            (p03, p13),
            (p10, p11),
            (p11, p12),
            (p12, p13),
            (p13, p10),
        ];

        lines.iter().for_each(|(a, b)| {
            commands.spawn(PolylineBundle {
                polyline: polylines.add(Polyline {
                    vertices: vec![a.cast::<f32>().into(), b.cast::<f32>().into()],
                }),
                material: polyline_materials.add(PolylineMaterial {
                    width: 1.0,
                    color: Color::WHITE.into(),
                    perspective: false,
                    ..default()
                }),
                transform,
                ..default()
            });
        });
    };

    for level in 0..lod {
        let ox = Vec3::X * level as f32 * bounds.size().x as f32;
        let transform = transform * Transform::from_translation(ox);
        spawn_bounding_box(
            &mut commands,
            &mut polylines,
            &mut polyline_materials,
            bounds.clone(),
            transform,
        );

        let coordinates = meta.coordinates().get(&level);
        if let Some(coordinates) = coordinates {
            coordinates.iter().for_each(|(k, bb)| {
                spawn_bounding_box(
                    &mut commands,
                    &mut polylines,
                    &mut polyline_materials,
                    bb.clone(),
                    transform,
                );

                let path = format!("{}/{}/{}.png", dir, level, k);
                let im = image::open(path).unwrap();
                let rgba = im.as_rgba8().unwrap();
                let points = rgba
                    .pixels()
                    .map(|pix| {
                        let [ix, iy, iz, _] = pix.0;
                        let fx = ix as f64 / 255.;
                        let fy = iy as f64 / 255.;
                        let fz = iz as f64 / 255.;
                        let v = Vector3::new(fx, fy, fz);
                        let p = bb.size().component_mul(&v) + bb.min().coords;
                        p.cast::<f32>().into()
                    })
                    .collect();
                commands.spawn(MaterialMeshBundle {
                    mesh: meshes.add(PointsMesh {
                        vertices: points,
                        ..Default::default()
                    }),
                    material: materials.add(PointsMaterial {
                        settings: PointsShaderSettings {
                            point_size: 1.0,
                            opacity: 1.0,
                            color: Color::WHITE.into(),
                            ..Default::default()
                        },
                        use_vertex_color: false,
                        perspective: true,
                        circle: true,
                        ..Default::default()
                    }),
                    transform,
                    ..Default::default()
                });

                // let f = File::open(path).unwrap();
                // let reader = BufReader::new(f);
                // let decoder = png::PngDecoder::new(reader).unwrap();
            });
        }
    }

    let scale = 5.;
    let camera = Camera3dBundle {
        projection: OrthographicProjection {
            scale,
            near: 1e-1,
            far: 1e4,
            scaling_mode: ScalingMode::FixedVertical(2.0),
            ..Default::default()
        }
        .into(),
        transform: Transform::from_translation(Vec3::new(0., 0., 10.))
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    };
    commands.spawn((camera, PanOrbitCamera::default()));
    commands.spawn(InfiniteGridBundle::default());
}

fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}
