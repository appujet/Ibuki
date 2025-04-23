use songbird::{Event, EventContext, EventHandler};
use songbird::id::GuildId;
use crate::IBUKI as Ibuki;

pub struct EndEvent {
    pub guild_id: GuildId
}

impl EventHandler for EndEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let event = ctx.to_owned();

        tokio::spawn(async move {
            if let EventContext::Track(_) = event {
                Ibuki.handles.remove(&self.guild_id);
                // track end
            }
        });

        None
    }
}

pub struct StartEvent {
    pub guild_id: GuildId
}

impl EventHandler for StartEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let event = ctx.to_owned();

        tokio::spawn(async move {
            if let EventContext::Track(_) = event {
                // track start
            }
        });

        None
    }
}

pub struct ErrorEvent {
    pub guild_id: GuildId
}

impl EventHandler for ErrorEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let event = ctx.to_owned();

        tokio::spawn(async move {
            if let EventContext::Track(_) = event {
                // track error
            }
        });

        None
    }
}

pub struct DriverDisconnectEvent {
    pub guild_id: GuildId
}

impl EventHandler for DriverDisconnectEvent {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let event = ctx.to_owned();

        tokio::spawn(async move {
            if let EventContext::Track(_) = event {
                Ibuki.connections.remove(&self.guild_id);
                Ibuki.handles.remove(&self.guild_id);
                // send data
            }
        });

        None
    }
}