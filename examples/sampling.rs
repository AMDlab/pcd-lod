use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_points::{
    material::PointsShaderSettings, mesh::PointsMesh, plugin::PointsPlugin, prelude::PointsMaterial,
};
use itertools::Itertools;
use nalgebra::Point3;
use pcd_lod::prelude::{Point, PoissonDiskSampling};

const RADIUS: f64 = 5.;
// const RADIUS: f64 = 20.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InfiniteGridPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(PointsPlugin)
        .add_plugins(AppPlugin)
        .run();
}
struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, update)
            .add_systems(Update, close_on_esc);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PointsMaterial>>,
) {
    let pcd = include_str!("../data/pcd.txt");
    let points = pcd
        .lines()
        .filter_map(|line| {
            let line = line.split_terminator(",").join(" ");
            Point::try_parse(&line).ok()
        })
        .collect_vec();

    // println!("points: {}", points.len());

    // project for debug
    let points = points
        .iter()
        .map(|pt| {
            let mut pt = pt.clone();
            pt.position = Point3::new(pt.position.x, pt.position.y, 0.);
            pt
        })
        .collect_vec();

    let center = points
        .iter()
        .map(|pt| pt.position)
        .fold(Point3::origin(), |a, b| (a.coords + b.coords).into())
        / points.len() as f64;

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(PointsMesh {
            vertices: points
                .iter()
                .map(|pt| (pt.position - center).cast::<f32>().into())
                .collect(),
            colors: Some(
                points
                    .iter()
                    .map(|pt| pt.color.unwrap_or_default().into())
                    .collect(),
            ),
        }),
        material: materials.add(PointsMaterial {
            settings: PointsShaderSettings {
                point_size: 0.175,
                opacity: 1.0,
                color: Color::WHITE.into(),
                ..Default::default()
            },
            use_vertex_color: true,
            perspective: true,
            circle: true,
            ..Default::default()
        }),
        ..Default::default()
    });

    let sampler = PoissonDiskSampling::default();

    let samples = sampler.sample(&points, RADIUS);
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(PointsMesh {
            vertices: samples
                .iter()
                .map(|pt| (pt.position - center).cast::<f32>().into())
                .collect(),
            colors: Some(
                samples
                    .iter()
                    .map(|pt| pt.color.unwrap_or_default().into())
                    .collect(),
            ),
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
        ..Default::default()
    });

    let scale = 5.;
    let orth = Camera3dBundle {
        projection: OrthographicProjection {
            scale,
            near: 1e-1,
            far: 1e4,
            scaling_mode: ScalingMode::FixedVertical(2.0),
            ..Default::default()
        }
        .into(),
        transform: Transform::from_translation(Vec3::new(0., 0., 5.))
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    };
    commands.spawn((orth, PanOrbitCamera::default()));
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

fn update(mut gizmos: Gizmos) {
    let grid = RADIUS / 3_f64.sqrt();
    let count = 100;
    let max = count as f32 * grid as f32;
    let oh = max / 2.;
    let color = Color::WHITE.with_alpha(0.25);
    for i in 0..=count {
        let v = i as f32 * grid as f32;
        gizmos.line(
            Vec3::new(-oh, v - oh, 0.),
            Vec3::new(max - oh, v - oh, 0.),
            color,
        );
        gizmos.line(
            Vec3::new(v - oh, -oh, 0.),
            Vec3::new(v - oh, max - oh, 0.),
            color,
        );
    }
}
