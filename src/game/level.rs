use bevy::{
    asset::{AssetLoader, AsyncReadExt},
    prelude::*,
    reflect::{TypePath, TypeUuid},
};
use bevy_pile::grid::Grid;
use bevy_simple_tilemap::prelude::*;
use bevy_simple_tilemap::TileFlags;
use serde::Deserialize;
use thiserror::Error;

use crate::cleanup::DependOnState;

use super::{
    collision::init_collision_map, level_select::CurrentLevel, player::SpawnPlayer, util::DIRS,
    GameAssets, GameState, TilePos,
};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimpleTileMapPlugin);
        app.register_type::<Level>();
        app.add_systems(
            OnTransition {
                from: GameState::LevelTransition,
                to: GameState::Play,
            },
            (spawn_level, apply_deferred)
                .chain()
                .before(init_collision_map),
        )
        .add_systems(
            Update,
            reload_on_change
                .run_if(in_state(GameState::Play))
                .run_if(on_event::<AssetEvent<Levels>>()),
        );
    }
}

#[derive(Component)]
pub struct LevelRoot;

fn spawn_level(
    mut cmds: Commands,
    current_level: Res<CurrentLevel>,
    levels_assets: Res<Assets<Levels>>,
    assets: Res<GameAssets>,
) {
    let levels_handle = &assets.levels;

    let levels = levels_assets
        .get(levels_handle)
        .expect("Level handle should be loaded");
    let level = levels
        .get(**current_level)
        .expect("Current level should only ever be set to a valid level");

    let level_root = cmds
        .spawn((
            SpatialBundle::default(),
            // DependOnState(vec![GameState::Play, GameState::Pause]),
            DependOnState::single(GameState::Play),
            Name::new("Level Root"),
            LevelRoot,
        ))
        .id();
    let tilemap_entity = cmds.spawn_empty().id();
    let mut tiles = Vec::new();
    for (idx, tile) in level.tiles.iter().enumerate() {
        let x = idx as i32 % level.size.x as i32;
        let y = idx as i32 / level.size.x as i32;
        let pos = TilePos(IVec2::new(x, y));

        let (index, flip) = tile.index_flip(&pos, level);

        // Tile is not walkable and above us is static tile
        /*
        if !tile.is_static() && level.tiles[idx + level.size.x as usize].is_static() {
            tiles.push((
                IVec3::new(x, y, 0),
                Some(Tile {
                    sprite_index: 14,
                    ..default()
                }),
            ));
        }
            */

        tiles.push((
            IVec3::new(x, y, 0),
            Some(Tile {
                sprite_index: index,
                flags: flip,
                ..default()
            }),
        ));

        match tile {
            TileKind::Wall => {}
            TileKind::Floor => {}
            TileKind::Player => cmds.add(SpawnPlayer {
                pos,
                tilemap_entity: level_root,
            }),
            TileKind::Pushable => {}
            TileKind::Pullable => {}
            TileKind::Platform => {}
            TileKind::Pit => {}
        }
    }

    let mut tilemap = TileMap::default();
    tilemap.set_tiles(tiles);
    let tilemap_entity = cmds
        .entity(tilemap_entity)
        .insert((
            TileMapBundle {
                tilemap,
                texture_atlas: assets.tiles.clone_weak(),
                ..default()
            },
            Name::new(format!("Level {}", **current_level)),
        ))
        .id();

    cmds.entity(level_root).add_child(tilemap_entity);
}

fn calculate_wall_index(pos: IVec2, level: &Level) -> (u32, TileFlags) {
    let level_grid = Grid::from_raw(level.size.as_ivec2(), level.tiles.clone());
    let [n, ne, e, se, s, sw, w, nw]: [bool; 8] = DIRS
        .iter()
        .map(|dir| {
            let npos = pos + *dir;
            level_grid.get(npos).map_or(false, |tile| !tile.is_static())
        })
        .collect::<Vec<bool>>()
        .try_into()
        .unwrap();

    let diag_c: usize = [ne, se, sw, nw].iter().map(|x| *x as usize).sum();
    let card_c: usize = [n, e, s, w].iter().map(|x| *x as usize).sum();

    #[derive(Default, Debug)]
    struct TileFlip {
        pub x: bool,
        pub y: bool,
        pub d: bool,
    }

    let flip_to_flag = |tileflip: TileFlip| -> TileFlags {
        let mut empty = TileFlags::empty();
        if tileflip.x {
            empty.insert(TileFlags::FLIP_X);
        }
        if tileflip.y {
            empty.insert(TileFlags::FLIP_Y);
        }
        if tileflip.d {
            empty.insert(TileFlags::FLIP_D);
        }
        empty
    };
    let flip = TileFlip {
        x: e,
        y: s,
        d: e || w,
    };
    let flip_inv = TileFlip {
        x: e,
        y: s,
        d: !e || !w,
    };
    let two_diag = TileFlip {
        x: ne,
        y: sw,
        d: (nw && sw) || (ne && se), // Both vertical
    };
    let zero_flip_diag = TileFlip {
        x: ne || se,
        y: sw || se,
        ..default()
    };
    let three_diag_flip = TileFlip {
        x: e || n && se || s && ne,
        y: s || e && sw || w && se,
        d: e || w,
    };
    let (id, flip) = match card_c {
        0 => match diag_c {
            0 => (8, TileFlip::default()),
            1 => (9, zero_flip_diag),
            2 => {
                if (nw && se) || (ne && sw) {
                    (11, two_diag)
                } else {
                    (10, two_diag)
                }
            }
            3 => (12, zero_flip_diag),
            4 => (13, TileFlip::default()),
            _ => unreachable!(),
        },
        1 => {
            if diag_c == 4 {
                (2, flip)
            } else if diag_c == 3 {
                (1, three_diag_flip)
            } else {
                (0, flip)
            }
        }
        2 => {
            if (n && s) || (w && e) {
                (5, flip_inv)
            } else if (n && w && se) || (n && e && sw) || (s && w && ne) || (s && e && nw) {
                (4, flip)
            } else {
                (3, flip)
            }
        }
        3 => (6, flip_inv),
        4 => (7, TileFlip::default()),
        _ => unreachable!(),
    };

    (id, flip_to_flag(flip))
}

fn reload_on_change(
    mut asset_events: EventReader<AssetEvent<Levels>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for ev in asset_events.read() {
        match ev {
            AssetEvent::Modified { id: _ } => {
                game_state.set(GameState::LevelTransition);
            }
            AssetEvent::Added { id: _ } => {}
            AssetEvent::Removed { id: _ } => {}
            AssetEvent::LoadedWithDependencies { id: _ } => {}
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Reflect)]
pub enum TileKind {
    Wall,
    Floor,
    Player,
    Pushable,
    Pullable,
    Platform,
    Pit,
}

impl TileKind {
    pub fn is_static(&self) -> bool {
        matches!(self, TileKind::Wall)
    }

    pub fn index_flip(&self, pos: &TilePos, level: &Level) -> (u32, TileFlags) {
        match self {
            TileKind::Floor | TileKind::Player | TileKind::Pushable | TileKind::Pullable => {
                (16, TileFlags::empty())
            }
            TileKind::Wall => calculate_wall_index(**pos, level),
            TileKind::Platform => (17, TileFlags::empty()),
            TileKind::Pit => (18, TileFlags::empty()),
        }
    }
}

impl From<u8> for TileKind {
    fn from(value: u8) -> Self {
        use TileKind::*;
        match value {
            b'#' => Wall,
            b'_' => Floor,
            b'@' => Player,
            b'b' => Pushable,
            b'p' => Pullable,
            b'-' => Platform,
            b'O' => Pit,
            _ => {
                bevy::log::warn!("Couldnt parse tile kind defaulting to wall tile");
                Wall
            }
        }
    }
}

#[derive(TypePath, TypeUuid, Debug, Deserialize, Deref, DerefMut, Asset)]
#[uuid = "39cadc56-aa9c-4543-8540-a018b74b5052"]
pub struct Levels(pub Vec<Level>);

#[derive(Debug, Deserialize)]
struct StringLevels(pub Vec<StringLevel>);

#[derive(Default)]
pub struct LevelLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum LevelLoaderError {
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse the ron: {0}")]
    RonError(#[from] ron::error::SpannedError),
}

impl AssetLoader for LevelLoader {
    type Asset = Levels;
    type Settings = ();
    type Error = LevelLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, std::result::Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let string_levels = ron::de::from_bytes::<StringLevels>(&bytes)?;

            let levels = string_levels
                .0
                .iter()
                .map(|string_level| {
                    let tiles = string_level
                        .tiles
                        .replace(['\n', ' '], "")
                        .as_bytes()
                        .iter()
                        .map(|byte| TileKind::from(*byte))
                        .collect::<Vec<TileKind>>()
                        .chunks_exact(string_level.size.x as usize)
                        .rev()
                        .flat_map(|chunk| chunk.to_vec())
                        .collect::<Vec<TileKind>>();
                    Level {
                        tiles,
                        size: string_level.size,
                    }
                })
                .collect::<Vec<Level>>();

            Ok(Levels(levels))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["levels"]
    }
}

#[derive(Deserialize, Debug, Reflect)]
pub struct Level {
    pub tiles: Vec<TileKind>,
    pub size: UVec2,
}

#[derive(Deserialize, Debug, Reflect)]
struct StringLevel {
    pub tiles: String,
    pub size: UVec2,
}
