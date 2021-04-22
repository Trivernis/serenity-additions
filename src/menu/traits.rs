use crate::error::Result;
use serenity::client::Context;
use serenity::http::Http;
use serenity::{async_trait, model::prelude::*};

#[async_trait]
pub trait EventDrivenMessage: Send + Sync {
    /// Returns if a message has been frozen and won't handle any further events
    fn is_frozen(&self) -> bool {
        false
    }

    /// Fired periodically
    async fn update(&mut self, _http: &Http) -> Result<()> {
        Ok(())
    }

    /// Fired when the message was deleted
    async fn on_deleted(&mut self, _ctx: &Context) -> Result<()> {
        Ok(())
    }

    /// Fired when a reaction was added to the message
    async fn on_reaction_add(&mut self, _ctx: &Context, _reaction: Reaction) -> Result<()> {
        Ok(())
    }

    /// Fired when a reaction was removed from the message
    async fn on_reaction_remove(&mut self, _ctx: &Context, _reaction: Reaction) -> Result<()> {
        Ok(())
    }
}
