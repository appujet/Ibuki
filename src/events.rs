use songbird::{CoreEvent, Event, EventContext, EventHandler, TrackEvent};
use songbird::id::GuildId;
use crate::manager::PlayerManager;

pub struct ManagerEvent {
    pub manager: PlayerManager,
    pub guild_id: GuildId,
    pub event_type: Event,
}

impl EventHandler for ManagerEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {

        tokio::spawn(async move {
            match self.event_type {
                Event::Periodic(_, _) => {
                    // todo: player update to ws
                }
                Event::Delayed(_) => {
                    // todo: track stuck to ws
                }
                Event::Track(event) => {
                    let _state = {
                        let EventContext::Track([(state, _)]) = ctx else {
                            None
                        };
                        Some(state)
                    };

                    match event {
                        TrackEvent::End => {
                            self.manager.delete_handle(self.guild_id);
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
                Event::Core(event) => {
                    if let CoreEvent::ClientDisconnect = event {
                        self.manager.delete_handle(self.guild_id);
                        // todo: driver disconnect and send to ws
                    }
                }
                _ => {},
            }

        });

        None
    }
}
