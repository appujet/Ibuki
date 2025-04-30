use crate::manager::PlayerManager;
use async_trait::async_trait;
use songbird::id::GuildId;
use songbird::{CoreEvent, Event, EventContext, EventHandler, TrackEvent};

pub struct ManagerEvent {
    pub manager: PlayerManager,
    pub guild_id: GuildId,
    pub event_type: Event,
}

#[async_trait]
impl EventHandler for ManagerEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let manager = self.manager.clone();
        let guild_id = self.guild_id;
        let event_type = self.event_type;

        let _state = match ctx {
            EventContext::Track([(state, _)]) => Some((*state).clone()),
            _ => None,
        };

        tokio::spawn(async move {
            match event_type {
                Event::Periodic(_, _) => {
                    // todo: player update to ws
                }
                Event::Delayed(_) => {
                    // todo: track stuck to ws
                }
                Event::Track(event) => {
                    match event {
                        TrackEvent::End => {
                            manager.delete_handle(guild_id);
                            // todo: track end and send to ws
                        }
                        TrackEvent::Playable => {
                            // todo: track start and send to ws
                        }
                        TrackEvent::Error => {
                            // todo: track error and send to ws also theres no error info here :aaa:
                        }
                        _ => {}
                    }
                }
                Event::Core(CoreEvent::ClientDisconnect) => {
                    manager.delete_handle(guild_id);
                    // todo: driver disconnect and send to ws
                }
                _ => {}
            }
        });

        None
    }
}
