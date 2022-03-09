use bevy::{prelude::*, math::vec3};
use bevy_ecs_ldtk::prelude::*;
use heron::prelude::*;

#[derive(Default, Component)]
pub struct MainCamera {
	pub last_cursor_position: Vec2,
}

#[derive(Default, Bundle)]
struct ColliderEntityBundle {
	pub collider: CollisionShape,
	pub rigid_body: RigidBody,
	pub velovity: Velocity,
	pub rotation_constraints: RotationConstraints,
	pub collision_layer: CollisionLayers,
//	pub physic_material: PhysicMaterial,
}

#[derive(PhysicsLayer)]
pub enum Layer {
	Player,
	Win,
}

impl From<EntityInstance> for ColliderEntityBundle{
	fn from(entity_instance: EntityInstance) -> Self {
		let rotation_constraints = RotationConstraints::lock();

		match entity_instance.identifier.as_ref() {
			"Player" => Self {
				collider: CollisionShape::Cuboid {
					half_extends: vec3(7., 14., 0.),
					border_radius: None,
				},
				rigid_body: RigidBody::Dynamic,
				rotation_constraints,
				collision_layer: CollisionLayers::none()
					.with_mask(Layer::Player)
					.with_groups([Layer::Win]),
				..Default::default()
			},
			"Win" =>{println!("win spawn"); Self {
				collider: CollisionShape::Cuboid {
					half_extends: vec3(8., 8., 0.),
					border_radius: None
				},
				rigid_body: RigidBody::Sensor,
				collision_layer: CollisionLayers::none()
					.with_mask(Layer::Win)
					.with_groups([Layer::Player]),
				..Default::default()
			}},
			"Enemy" => Self {
				collider: CollisionShape::Cuboid {
					half_extends: vec3(8., 8., 0.),
					border_radius: None
				},
				rigid_body: RigidBody::Dynamic,
				//rotation_constraints,
				..Default::default()
			},
			_ => Self {
				collision_layer: CollisionLayers::none(),
				..Default::default()
			}
		}
	}
}

#[derive(Default, Bundle)]
struct TimerBundle {
	timer: Timer,
}

impl From<EntityInstance> for TimerBundle {
	fn from(entity_instance: EntityInstance) -> Self {
		match entity_instance.identifier.as_ref() {
			"Player" => Self { timer: Timer::from_seconds(1./8., true) },
			"Enemy"  => Self { timer: Timer::from_seconds(1./12., true) },
			_ => Self::default()
		}
	}
}

#[derive(Default, Component)]
pub struct Player;

#[derive(Bundle, LdtkEntity)]
pub struct PlayerBundle {
	pub player: Player,
	#[from_entity_instance]
	#[bundle]
	collider: ColliderEntityBundle,
	#[sprite_sheet_bundle("texture/player.png", 32.0, 32.0, 6, 1, 0.0, 0)]
	#[bundle]
	pub sprite_sheet_bundle: SpriteSheetBundle,
	#[from_entity_instance]
	#[bundle]
	timer_bundle: TimerBundle,
}

#[derive(Default, Component)]
pub struct Enemy;

#[derive(Bundle, LdtkEntity)]
pub struct EnemyBundle {
	#[from_entity_instance]
	#[bundle]
	collider: ColliderEntityBundle,
	#[sprite_sheet_bundle("texture/enemy.png", 32.0, 32.0, 4, 1, 0.0, 0)]
	#[bundle]
	sprite_sheet_bundle: SpriteSheetBundle,
	enemy: Enemy,
	#[from_entity_instance]
	#[bundle]
	timer_bundle: TimerBundle,
}


#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component)]
pub struct Wall;

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct WallBundle {
	wall: Wall,
}

#[derive(Default, Component)]
pub struct Win;

#[derive(Bundle, LdtkEntity)]
pub struct WinBundle {
	#[from_entity_instance]
	#[bundle]
	collider_bundle: ColliderEntityBundle,
	#[sprite_bundle("texture/meta.png")]
	#[bundle]
	pub sprite_bundle: SpriteBundle,

	win: Win,
}

