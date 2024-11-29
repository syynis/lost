use std::marker::PhantomData;

use bevy::{
    app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*,
    state::state::FreelyMutableState,
};

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct BeforeStateTransition;

pub fn cleanup_all_with<T: Component>(mut cmds: Commands, query: Query<Entity, With<T>>) {
    query
        .iter()
        .for_each(|e| cmds.entity(e).despawn_recursive());
}

#[derive(Default)]
pub struct StateCleanupPlugin<S: States + FreelyMutableState> {
    phantom: PhantomData<S>,
}

impl<S: States + FreelyMutableState> Plugin for StateCleanupPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_schedule(Schedule::new(BeforeStateTransition));
        app.add_systems(BeforeStateTransition, cleanup_on_state_change::<S>);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_before(StateTransition, BeforeStateTransition);
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct DependOnState<T: States + FreelyMutableState>(pub Vec<T>);

impl<T: States + FreelyMutableState> DependOnState<T> {
    pub fn single(state: T) -> Self {
        Self(vec![state])
    }
}

fn cleanup_on_state_change<T: States + FreelyMutableState>(
    mut cmds: Commands,
    query: Query<(Entity, &DependOnState<T>)>,
    next_state: Res<NextState<T>>,
    current_state: Res<State<T>>,
    names: Query<&Name>,
) {
    let NextState::Pending(next_state) = next_state.into_inner() else {
        return;
    };

    for (entity, on_state) in query.iter() {
        if !on_state.contains(next_state) {
            cmds.entity(entity).despawn_recursive();
            bevy::log::debug!(
                "Cleanup {} in {:?} to {:?}",
                names
                    .get(entity)
                    .map_or(format!("{:?}", entity), |name| name.to_string()),
                **current_state,
                next_state
            );
        }
    }
}
