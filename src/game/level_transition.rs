use std::time::Duration;

use bevy::prelude::*;

use crate::{
    cleanup::DependOnState,
    event_scheduler::{EventScheduler, EventSchedulerPlugin},
};

use super::{level_select::CurrentLevel, GameState};

const CURRENT_STATE: GameState = GameState::LevelTransition;

pub struct LevelTransitionPlugin;

impl Plugin for LevelTransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EventSchedulerPlugin::<LevelTransitionEvent>::default())
            .add_systems(OnEnter(CURRENT_STATE), spawn_level_card)
            .add_systems(Update, transition.run_if(in_state(CURRENT_STATE)));
    }
}

#[derive(Component)]
struct LevelCard;

#[derive(Event)]
enum LevelTransitionEvent {
    End,
}

fn spawn_level_card(
    mut cmds: Commands,
    current_level: Res<CurrentLevel>,
    mut level_transition_scheduler: ResMut<EventScheduler<LevelTransitionEvent>>,
) {
    cmds.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                align_content: AlignContent::Center,
                margin: UiRect::all(Val::Auto),
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        },
        DependOnState::single(CURRENT_STATE),
    ))
    .with_children(|parent| {
        parent.spawn(TextBundle {
            style: Style {
                width: Val::Auto,
                height: Val::Px(65.),
                margin: UiRect {
                    top: Val::Px(10.),
                    bottom: Val::Px(10.),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(4.)),
                ..default()
            },
            text: Text::from_section(
                format!("Level {}", **current_level),
                TextStyle {
                    font_size: 48.,
                    color: Color::BLACK,
                    ..default()
                },
            ),
            ..default()
        });
    })
    .with_children(|parent| {
        parent.spawn(TextBundle {
            style: Style {
                width: Val::Auto,
                height: Val::Px(65.),
                margin: UiRect {
                    top: Val::Px(10.),
                    bottom: Val::Px(10.),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(4.)),
                ..default()
            },
            text: Text::from_section(
                "Press any key to continue",
                TextStyle {
                    font_size: 28.,
                    color: Color::BLACK,
                    ..default()
                },
            ),
            ..default()
        });
    });
    level_transition_scheduler.schedule(LevelTransitionEvent::End, Duration::from_millis(50));
}

fn transition(
    level_transition: EventReader<LevelTransitionEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !level_transition.is_empty() {
        next_state.set(GameState::Play)
    }
}
