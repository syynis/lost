use bevy::{ecs::system::Command, prelude::*};
use leafwing_input_manager::prelude::*;

use super::{
    history::{HandleHistoryEvents, History},
    Dir, GameAssets, GameState, TilePos,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerMove>();
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

#[derive(Event)]
pub struct PlayerMove(pub Dir);

pub fn player_movement(
    mut player_q: Query<&mut MovementTimer, With<Player>>,
    player_actions: Query<&ActionState<PlayerActions>>,
    mut sokoban_events: EventWriter<PlayerMove>,
    time: Res<Time>,
) {
    let Ok(mut movement_timer) = player_q.get_single_mut() else {
        return;
    };

    let player_actions = player_actions
        .get_single()
        .expect("Player input map should exist");

    movement_timer.tick(time.delta());

    if !movement_timer.finished() {
        return;
    }

    for direction in player_actions
        .get_pressed()
        .iter()
        .map(|action| Dir::from(*action))
    {
        movement_timer.reset();
        sokoban_events.send(PlayerMove(direction));
    }
}
