use std::{marker::PhantomData, ops::AddAssign};

use bevy::prelude::*;

use super::GameState;

pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTime>()
            .register_type::<CurrentTime>()
            .add_event::<HistoryEvent>()
            .add_systems(
                OnTransition {
                    from: GameState::LevelTransition,
                    to: GameState::Play,
                },
                reset_time,
            )
            .add_systems(Update, handle_time.in_set(HandleHistoryEvents));
    }
}

#[derive(Default)]
pub struct HistoryComponentPlugin<C: Component + Clone> {
    phantom: PhantomData<C>,
}

impl<C: Component + Clone> Plugin for HistoryComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_history_events::<C>
                .in_set(HandleHistoryEvents)
                .before(handle_time),
        );
    }
}

#[derive(Default)]
pub struct PreviousComponentPlugin<C: Component + Clone> {
    phantom: PhantomData<C>,
}

impl<C: Component + Clone> Plugin for PreviousComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_previous::<C>
                .in_set(HandlePreviousComponent)
                .after(HandleHistoryEvents),
        );
    }
}

#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct HandleHistoryEvents;

#[derive(SystemSet, Copy, Clone, Eq, PartialEq, Debug, Default, Hash)]
pub struct HandlePreviousComponent;

#[derive(Resource, Reflect, Default, Copy, Clone, Debug, Deref, DerefMut)]
#[reflect(Resource)]
pub struct CurrentTime(pub usize);

#[derive(Event)]
pub enum HistoryEvent {
    Record,
    Rewind,
    Reset,
}

fn reset_time(mut current_time: ResMut<CurrentTime>) {
    **current_time = 0;
}

fn handle_time(
    mut current_time: ResMut<CurrentTime>,
    mut history_events: EventReader<HistoryEvent>,
) {
    for ev in history_events.read() {
        match ev {
            HistoryEvent::Record => current_time.add_assign(1),
            HistoryEvent::Rewind => **current_time = current_time.saturating_sub(1),
            HistoryEvent::Reset => current_time.add_assign(1),
        }
    }
}

#[derive(Bundle, Default)]
pub struct HistoryBundle<C: Component + Clone> {
    history: History<C>,
    previous: PreviousComponent<C>,
}

#[derive(Component, Clone, Default, Deref, DerefMut, Reflect)]
pub struct History<C: Component + Clone>(Vec<(usize, C)>);

#[derive(Component, Clone, Default, Deref, DerefMut, Reflect)]
pub struct PreviousComponent<C: Component + Clone>(C);

impl<C: Component + Clone> PreviousComponent<C> {
    pub fn get(&self) -> &C {
        self
    }
}

fn update_previous<C: Component + Clone>(
    mut components: Query<(&C, &mut PreviousComponent<C>), Changed<C>>,
) {
    for (component, mut previous) in components.iter_mut() {
        **previous = component.clone();
    }
}

fn handle_history_events<C>(
    mut history_query: Query<(&mut History<C>, &mut C, &PreviousComponent<C>)>,
    mut history_events: EventReader<HistoryEvent>,
    current_time: Res<CurrentTime>,
) where
    C: Component + Clone,
{
    for ev in history_events.read() {
        match ev {
            HistoryEvent::Record => {
                for (mut history, _, prev) in history_query.iter_mut() {
                    history.push((**current_time, prev.get().clone()));
                }
            }
            HistoryEvent::Rewind => {
                for (mut history, mut component, _) in history_query.iter_mut() {
                    if let Some((t, _)) = history.last() {
                        if (t + 1) == **current_time {
                            let (_, prev_component) = history.pop().unwrap();
                            *component = prev_component;
                        }
                    }
                }
            }
            HistoryEvent::Reset => {
                for (mut history, mut component, _) in history_query.iter_mut() {
                    if let Some(first) = history.first() {
                        let first_component = first.1.clone();
                        history.push((**current_time, component.clone()));
                        *component = first_component;
                    }
                }
            }
        }
    }
}
