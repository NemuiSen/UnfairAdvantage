mod components;
mod systems;

use bevy::{
	input::system::exit_on_esc_system,
	prelude::*, render::options::{WgpuOptions, WgpuLimits},
};
use bevy_ecs_ldtk::prelude::*;
use heron::prelude::*;


/*
 * El jugador esta en un campo vacio y obsccuro donde tiene que enfrentarse a enemigos
 * que lo pueden ver atravez de la obscuridad y tambien tienen la facilidad de lasrimarte.
 * Tu objetivo es escapar de ahi sin morir.
 * Tienes una arma que puede iluminar una zona y llamar la atencion de los monstruos
 *
 */

fn main() {
	App::new()
		.insert_resource(WgpuOptions {
			limits: WgpuLimits {
				max_texture_array_layers: 2048,
				..Default::default()
			},
			..Default::default()
		})
		.add_plugins(DefaultPlugins)
		.add_plugin(LdtkPlugin)
		.add_plugin(PhysicsPlugin::default())
		.insert_resource(LevelSelection::Uid(0))
		.add_startup_system(systems::setup)
		.add_system(exit_on_esc_system)
		.add_system(systems::movement)
		.add_system(systems::camera_cursor_position)
		.add_system(systems::camera_controller)
		.add_system(systems::animation)
		.add_system(systems::pause_physics_during_load)
		.add_system(systems::win)
		.add_system(systems::spawn_wall_collision)
		.add_system(systems::enemy_movement)
		.register_ldtk_entity::<components::PlayerBundle>("Player")
		.register_ldtk_entity::<components::EnemyBundle>("Enemy")
		.register_ldtk_entity::<components::WinBundle>("Win")
		.register_ldtk_int_cell::<components::WallBundle>(1)
		.run();
}

