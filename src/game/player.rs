use std::ops::AddAssign;

use bevy::{ecs::system::Command, log, prelude::*};
use leafwing_input_manager::prelude::*;

use super::{
    collision::CollisionMap,
    history::{HandleHistoryEvents, History},
    Dir, EntityKind, GameAssets, GameState, TilePos,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerActions>::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                player_movement
                    .before(HandleHistoryEvents)
                    .run_if(in_state(GameState::Play)),
            );
    }
}

#[derive(Component, Clone)]
pub struct Player;

#[derive(Actionlike, Clone, Copy, Hash, Debug, PartialEq, Eq, Reflect)]
pub enum PlayerActions {
    Up,
    Right,
    Down,
    Left,
}

impl From<PlayerActions> for Dir {
    fn from(value: PlayerActions) -> Dir {
        match value {
            PlayerActions::Up => Dir::Up,
            PlayerActions::Left => Dir::Left,
            PlayerActions::Down => Dir::Down,
            PlayerActions::Right => Dir::Right,
        }
    }
}

pub struct SpawnPlayer {
    pub pos: TilePos,
    pub tilemap_entity: Entity,
}

impl SpawnPlayer {
    pub fn new(pos: TilePos, tilemap_entity: Entity) -> Self {
        Self {
            pos,
            tilemap_entity,
        }
    }
}

impl Command for SpawnPlayer {
    fn apply(self, world: &mut World) {
        let texture = world.resource::<GameAssets>().player.clone();
        world
            .entity_mut(self.tilemap_entity)
            .with_children(|child_builder| {
                child_builder.spawn((
                    Name::new("Player"),
                    Player,
                    self.pos,
                    History::<TilePos>::default(),
                    SpriteBundle {
                        texture,
                        transform: Transform::from_translation(2. * Vec3::Z),
                        ..default()
                    },
                    MovementTimer::default(),
                    EntityKind::Pushable,
                ));
            });
    }
}

fn setup(mut cmds: Commands) {
    cmds.spawn((
        (InputManagerBundle::<PlayerActions> {
            input_map: player_actions(),
            ..default()
        },),
        Name::new("PlayerActions"),
    ));
}

fn player_actions() -> InputMap<PlayerActions> {
    use PlayerActions::*;
    let mut input_map = InputMap::default();

    input_map.insert(KeyCode::W, Up);
    input_map.insert(KeyCode::D, Right);
    input_map.insert(KeyCode::S, Down);
    input_map.insert(KeyCode::A, Left);

    input_map
}

#[derive(Clone, Debug, Component, Deref, DerefMut)]
pub struct MovementTimer(pub Timer);

impl Default for MovementTimer {
    fn default() -> MovementTimer {
        MovementTimer(Timer::from_seconds(0.075, TimerMode::Once))
    }
}

pub fn player_movement(
    mut player_q: Query<(Entity, &mut MovementTimer), With<Player>>,
    mut dynamic_entities: Query<&mut TilePos>,
    player_actions: Query<&ActionState<PlayerActions>>,
    time: Res<Time>,
    collision: Res<CollisionMap>,
) {
    let Ok((player_entity, mut movement_timer)) = player_q.get_single_mut() else {
        return;
    };

    let player_actions = player_actions
        .get_single()
        .expect("Player input map should exist");

    movement_timer.tick(time.delta());

    if !movement_timer.finished() {
        return;
    }

    let player_pos = dynamic_entities
        .get(player_entity)
        .expect("Player always has tile pos")
        .0
        .clone();
    for direction in player_actions
        .get_pressed()
        .iter()
        .map(|action| Dir::from(*action))
    {
        movement_timer.reset();

        match collision.push_collision(player_pos, direction) {
            super::collision::CollisionResult::Push(push) => {
                let dir_vec = IVec2::from(direction);
                for e in push {
                    dynamic_entities
                        .get_component_mut::<TilePos>(e)
                        .expect("Every entity in collision map has tile pos")
                        .add_assign(dir_vec);
                }
            }
            super::collision::CollisionResult::Collision => {
                log::debug!("Can't move");
            }
            super::collision::CollisionResult::OutOfBounds => {
                log::debug!("Player out of bounds");
            }
        }
    }
}
