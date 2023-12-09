use std::{collections::VecDeque, marker::PhantomData, time::Duration};

use bevy::prelude::*;

pub struct EventSchedulerPlugin<E: Event> {
    phantom: PhantomData<E>,
}

impl<E: Event> Plugin for EventSchedulerPlugin<E>
where
    E: 'static + Send + Sync,
{
    fn build(&self, app: &mut App) {
        app.add_event::<E>()
            .init_resource::<EventScheduler<E>>()
            .add_systems(Update, send_scheduled_events::<E>);
    }
}

impl<E: Event> Default for EventSchedulerPlugin<E>
where
    E: 'static + Sync + Send,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct EventScheduler<E: Event>
where
    E: 'static + Send + Sync,
{
    events: VecDeque<(E, Timer)>,
}

impl<E: Event> Default for EventScheduler<E>
where
    E: 'static + Send + Sync,
{
    fn default() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }
}

impl<E: Event> EventScheduler<E>
where
    E: 'static + Send + Sync,
{
    pub fn schedule(&mut self, event: E, duration: Duration) {
        self.events
            .push_back((event, Timer::new(duration, TimerMode::Once)))
    }
}

fn send_scheduled_events<E: Event>(
    time: Res<Time>,
    mut event_scheduler: ResMut<EventScheduler<E>>,
    mut writer: EventWriter<E>,
) where
    E: 'static + Send + Sync,
{
    event_scheduler.events = event_scheduler
        .events
        .drain(..)
        .filter_map(|(event, mut timer)| {
            timer.tick(time.delta());
            if timer.finished() {
                writer.send(event);
                None
            } else {
                Some((event, timer))
            }
        })
        .collect()
}
