use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use heron::prelude::*;

use crate::components::*;

pub fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	commands.spawn_bundle(OrthographicCameraBundle {
		orthographic_projection: OrthographicProjection {
			scale: 1./2.,
			..Default::default()
		},
		..OrthographicCameraBundle::new_2d()
	}).insert(MainCamera::default());
	commands.spawn_bundle(UiCameraBundle::default());
	commands.spawn_bundle(LdtkWorldBundle {
		ldtk_handle: asset_server.load("tilemap/main.ldtk"),
		..Default::default()
	});
}

pub fn pause_physics_during_load(
	mut level_events: EventReader<LevelEvent>,
	mut physics_time: ResMut<PhysicsTime>,
) {
	for event in level_events.iter() {
		match event {
			LevelEvent::SpawnTriggered(_) => physics_time.set_scale(0.),
			LevelEvent::Transformed(_) => physics_time.set_scale(1.),
			_ => (),
		}
	}
}

/// Spawns heron collisions for the walls of a level
///
/// You could just insert a ColliderBundle in to the WallBundle,
/// but this spawns a different collider for EVERY wall tile.
/// This approach leads to bad performance.
///
/// Instead, by flagging the wall tiles and spawning the collisions later,
/// we can minimize the amount of colliding entities.
///
/// The algorithm used here is a nice compromise between simplicity, speed,
/// and a small number of rectangle colliders.
/// In basic terms, it will:
/// 1. consider where the walls are
/// 2. combine wall tiles into flat "plates" in each individual row
/// 3. combine the plates into rectangles across multiple rows wherever possible
/// 4. spawn colliders for each rectangle
pub fn spawn_wall_collision(
	mut commands: Commands,
	wall_query: Query<(&GridCoords, &Parent), Added<Wall>>,
	parent_query: Query<&Parent, Without<Wall>>,
	level_query: Query<(Entity, &Handle<LdtkLevel>)>,
	levels: Res<Assets<LdtkLevel>>,
) {
	/// Represents a wide wall that is 1 tile tall
	/// Used to spawn wall collisions
	#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
	struct Plate {
		left: i32,
		right: i32,
	}

	// consider where the walls are
	// storing them as GridCoords in a HashSet for quick, easy lookup
	let mut level_to_wall_locations: HashMap<Entity, HashSet<GridCoords>> = HashMap::new();

	wall_query.for_each(|(&grid_coords, &Parent(parent))| {
		// the intgrid tiles' direct parents will be bevy_ecs_tilemap chunks, not the level
		// To get the level, you need their grandparents, which is where parent_query comes in
		if let Ok(&Parent(level_entity)) = parent_query.get(parent) {
			level_to_wall_locations
				.entry(level_entity)
				.or_insert(HashSet::new())
				.insert(grid_coords);
		}
	});

	if !wall_query.is_empty() {
		level_query.for_each(|(level_entity, level_handle)| {
			if let Some(level_walls) = level_to_wall_locations.get(&level_entity) {
				let level = levels
					.get(level_handle)
					.expect("Level should be loaded by this point");

				let LayerInstance {
					c_wid: width,
					c_hei: height,
					grid_size,
					..
				} = level
					.level
					.layer_instances
					.clone()
					.expect("Level asset should have layers")[0];

				// combine wall tiles into flat "plates" in each individual row
				let mut plate_stack: Vec<Vec<Plate>> = Vec::new();

				for y in 0..height {
					let mut row_plates: Vec<Plate> = Vec::new();
					let mut plate_start = None;

					// + 1 to the width so the algorithm "terminates" plates that touch the right
					// edge
					for x in 0..width + 1 {
						match (plate_start, level_walls.contains(&GridCoords { x, y })) {
							(Some(s), false) => {
								row_plates.push(Plate {
									left: s,
									right: x - 1,
								});
								plate_start = None;
							}
							(None, true) => plate_start = Some(x),
							_ => (),
						}
					}

					plate_stack.push(row_plates);
				}

				// combine "plates" into rectangles across multiple rows
				let mut wall_rects: Vec<Rect<i32>> = Vec::new();
				let mut previous_rects: HashMap<Plate, Rect<i32>> = HashMap::new();

				// an extra empty row so the algorithm "terminates" the rects that touch the top
				// edge
				plate_stack.push(Vec::new());

				for (y, row) in plate_stack.iter().enumerate() {
					let mut current_rects: HashMap<Plate, Rect<i32>> = HashMap::new();
					for plate in row {
						if let Some(previous_rect) = previous_rects.remove(&plate) {
							current_rects.insert(
								*plate,
								Rect {
									top: previous_rect.top + 1,
									..previous_rect
								},
							);
						} else {
							current_rects.insert(
								*plate,
								Rect {
									bottom: y as i32,
									top: y as i32,
									left: plate.left,
									right: plate.right,
								},
							);
						}
					}

					// Any plates that weren't removed above have terminated
					wall_rects.append(&mut previous_rects.values().copied().collect());
					previous_rects = current_rects;
				}

				// spawn colliders for every rectangle
				for wall_rect in wall_rects {
					commands
						.spawn()
						.insert(CollisionShape::Cuboid {
							half_extends: Vec3::new(
								(wall_rect.right as f32 - wall_rect.left as f32 + 1.)
									* grid_size as f32
									/ 2.,
								(wall_rect.top as f32 - wall_rect.bottom as f32 + 1.)
									* grid_size as f32
									/ 2.,
								0.,
							),
							border_radius: None,
						})
						.insert(RigidBody::Static)
						.insert(PhysicMaterial {
							friction: 0.1,
							..Default::default()
						})
						.insert(Transform::from_xyz(
							(wall_rect.left + wall_rect.right + 1) as f32 * grid_size as f32 / 2.,
							(wall_rect.bottom + wall_rect.top + 1) as f32 * grid_size as f32 / 2.,
							0.,
						))
						.insert(GlobalTransform::default())
						// Making the collider a child of the level serves two purposes:
						// 1. Adjusts the transforms to be relative to the level for free
						// 2. the colliders will be despawned automatically when levels unload
						.insert(Parent(level_entity));
				}
			}
		});
	}
}

pub fn enemy_movement(
	player_query: Query<&Transform, With<Player>>,
	mut enemy_query: Query<(&mut Velocity, &Transform), With<Enemy>>
) {
	if let Ok(Transform { translation: player_translation, .. }) = player_query.get_single() {
		for (mut enemy_velocity, Transform { translation: enemy_translation, .. }) in enemy_query.iter_mut() {
			let delta = *player_translation - *enemy_translation;
			if delta.length() < 200.0 {
				enemy_velocity.linear = delta.normalize_or_zero() * 90.0;
			}
		}
	}
}

pub fn movement(
	input: Res<Input<KeyCode>>,
	mut query: Query<&mut Velocity, With<Player>>,
) {
	if let Ok(mut velocity) = query.get_single_mut() {
		let mut delta = Vec2::ZERO;
		if input.pressed(KeyCode::W) { delta.y += 1.0 }
		if input.pressed(KeyCode::A) { delta.x -= 1.0 }
		if input.pressed(KeyCode::S) { delta.y -= 1.0 }
		if input.pressed(KeyCode::D) { delta.x += 1.0 }
		delta = delta.normalize_or_zero();

		velocity.linear = delta.extend(0.0) * 100.;
	}
}

pub fn animation(
	time: Res<Time>,
	texture_atlases: Res<Assets<TextureAtlas>>,
	mut query: Query<(&Velocity, &mut Timer, &mut TextureAtlasSprite, &Handle<TextureAtlas>)>,
) {
	for (velocity, mut timer, mut sprite, texture_atlas_handle) in query.iter_mut() {
		timer.tick(time.delta());
		if timer.finished() && velocity.linear != Vec3::ZERO {
			let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
			sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
		} else if velocity.linear == Vec3::ZERO {
			sprite.index = 0;
		}
	}
}

pub fn win(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut physic_event: EventReader<CollisionEvent>,
) {
	physic_event.iter().filter(|e| e.is_started()).filter_map(|event| {
		let (e1, e2) = event.rigid_body_entities();
		let (l1, l2) = event.collision_layers();

		if l1.contains_group(Layer::Player) && !l1.contains_group(Layer::Win) && l2.contains_group(Layer::Win) && !l2.contains_group(Layer::Player) {
			Some(e2)
		} else if l2.contains_group(Layer::Win) && !l2.contains_group(Layer::Player) && l1.contains_group(Layer::Player) && !l1.contains_group(Layer::Win) {
			Some(e1)
		} else {
			None
		}
	}).for_each(|entity_win| {
		commands.entity(entity_win).despawn();
		commands.spawn_bundle(TextBundle {
				style: Style {
					margin: Rect::all(Val::Px(5.0)),
					..Default::default()
				},
				text: Text::with_section(
					"You Win!!!",
					TextStyle {
						font: asset_server.load("fonts/FiraSans-Bold.ttf"),
						font_size: 100.0,
						color: Color::WHITE,
					},
				Default::default(),
			),
			..Default::default()
		});
	})
}

// Memoriza la ultima posicion del mouse
pub fn camera_cursor_position(
	wnds: Res<Windows>,
	mut cursor_moved: EventReader<CursorMoved>,
	mut mc_query: Query<(&mut MainCamera, &OrthographicProjection)>,
) {
	let wnd = wnds.get_primary().unwrap();
	let (mut mc, op) = mc_query.single_mut();
	if let Some(cm) = cursor_moved.iter().last() {
		let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
		let p = cm.position - size / 2.0;
		mc.last_cursor_position = p * op.scale;
	}
}

// Camara fachera
pub fn camera_controller(
	mut player_query: Query<(&Transform, &mut TextureAtlasSprite), With<Player>>,
	mut camera_query: Query<(&mut Transform, &MainCamera), Without<Player>>
) {
	if let Ok((Transform { translation: player_translation, .. }, mut sprite)) = player_query.get_single_mut() {
		let (mut camera_trans, mc) = camera_query.single_mut();
		camera_trans.translation = *player_translation + mc.last_cursor_position.extend(0.0) / 2.0;

		if mc.last_cursor_position.x >= 0.0 {
			sprite.flip_x = false;
		} else {
			sprite.flip_x = true;
		}
	}
}

