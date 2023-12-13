use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use leafwing_input_manager::prelude::*;

use self::{
    history::{History, HistoryEvent, PreviousComponent},
    level::{LevelLoader, Levels},
};

pub mod collision;
pub mod history;
pub mod level;
pub mod level_select;
pub mod level_transition;
pub mod mechanics;
pub mod player;
pub mod util;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            InputManagerPlugin::<GameAction>::default(),
            level_select::LevelSelectPlugin,
            level_transition::LevelTransitionPlugin,
            player::PlayerPlugin,
            collision::CollisionPlugin,
            level::LevelPlugin,
            history::HistoryPlugin,
            history::HistoryComponentPlugin::<TilePos>::default(),
            history::PreviousComponentPlugin::<TilePos>::default(),
            mechanics::MechanicsPlugin,
        ));
        app.register_asset_loader(LevelLoader)
            .init_asset::<Levels>();
        app.register_type::<TilePos>()
            .register_type::<Dir>()
            .register_type::<History<TilePos>>()
            .register_type::<PreviousComponent<TilePos>>()
            .register_type::<EntityKind>();
        app.add_state::<GameState>()
            .add_loading_state(
                LoadingState::new(GameState::AssetLoading)
                    .continue_to_state(GameState::LevelSelect),
            )
            .add_collection_to_loading_state::<_, GameAssets>(GameState::AssetLoading);
        app.add_systems(Startup, setup)
            .add_systems(Update, history.run_if(in_state(GameState::Play)))
            .add_systems(
                StateTransition,
                crate::cleanup::cleanup_on_state_change::<GameState>
                    .before(apply_state_transition::<GameState>),
            )
            .add_systems(PostUpdate, copy_pos_to_transform);
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    AssetLoading,
    LevelSelect,
    LevelTransition,
    Play,
}

#[derive(Resource, Default, AssetCollection, Debug)]
pub struct GameAssets {
    #[asset(path = "player.png")]
    pub player: Handle<Image>,
    #[asset(path = "pushable.png")]
    pub pushable: Handle<Image>,
    #[asset(path = "pullable.png")]
    pub pullable: Handle<Image>,
    #[asset(texture_atlas(tile_size_x = 8., tile_size_y = 8., columns = 8, rows = 3))]
    #[asset(path = "tiles.png")]
    pub tiles: Handle<TextureAtlas>,
    #[asset(path = "button.png")]
    pub button: Handle<Image>,
    #[asset(path = "test.levels.ron")]
    pub levels: Handle<Levels>,
}

pub fn copy_pos_to_transform(mut query: Query<(&TilePos, &mut Transform), Changed<TilePos>>) {
    for (pos, mut transform) in query.iter_mut() {
        let new = pos.wpos().extend(transform.translation.z);

        transform.translation = new;
    }
}

fn setup(mut cmds: Commands) {
    cmds.spawn((
        (InputManagerBundle::<GameAction> {
            input_map: game_actions(),
            ..default()
        },),
        Name::new("GameActions"),
    ));
}

fn history(
    actions: Query<&ActionState<GameAction>>,
    mut history_events: EventWriter<HistoryEvent>,
) {
    let Ok(actions) = actions.get_single() else {
        return;
    };
    if actions.just_pressed(GameAction::Undo) {
        history_events.send(HistoryEvent::Rewind);
    } else if actions.just_pressed(GameAction::Reset) {
        history_events.send(HistoryEvent::Reset);
    }
}

#[derive(Actionlike, Clone, Copy, Hash, Debug, PartialEq, Eq, Reflect)]
pub enum GameAction {
    Undo,
    Reset,
}

fn game_actions() -> InputMap<GameAction> {
    use GameAction::*;
    let mut input_map = InputMap::default();

    input_map.insert(KeyCode::E, Undo);
    input_map.insert(KeyCode::R, Reset);

    input_map
}

#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Deref, DerefMut, Reflect)]
pub struct TilePos(pub IVec2);

impl TilePos {
    pub fn new(x: i32, y: i32) -> Self {
        TilePos(IVec2::new(x, y))
    }

    pub fn add_dir(&mut self, dir: Dir) {
        self.0 += IVec2::from(dir);
    }

    pub fn wpos(&self) -> Vec2 {
        self.as_vec2() * 8.
    }
}

#[derive(Debug, Copy, Clone, Reflect)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left,
}

impl Dir {
    pub fn opposite(&self) -> Dir {
        use Dir::*;
        match self {
            Up => Down,
            Down => Up,
            Left => Right,
            Right => Left,
        }
    }
}

impl From<Dir> for IVec2 {
    fn from(direction: Dir) -> IVec2 {
        match direction {
            Dir::Up => IVec2::Y,
            Dir::Left => IVec2::NEG_X,
            Dir::Down => IVec2::NEG_Y,
            Dir::Right => IVec2::X,
        }
    }
}

#[derive(Debug, Copy, Clone, Component, Reflect, PartialEq)]
pub enum EntityKind {
    Wall,
    Pit,
    Platform,
    Pullable,
    Pushable,
}
