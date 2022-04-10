use crate::core::{BoxedMessage, MessageHandle};
use crate::Error;
use crate::Result;
use dashmap::DashMap;
use serenity::client::Context;
use serenity::prelude::TypeMapKey;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Container to store event driven messages in the serenity context data
pub struct EventDrivenMessageContainer;
pub type MessageRef = Arc<Mutex<BoxedMessage>>;
pub type EventDrivenMessagesRef = Arc<DashMap<MessageHandle, MessageRef>>;

impl TypeMapKey for EventDrivenMessageContainer {
    type Value = EventDrivenMessagesRef;
}

#[tracing::instrument(level = "trace", skip(ctx))]
pub async fn get_listeners_from_context(ctx: &Context) -> Result<EventDrivenMessagesRef> {
    let data = ctx.data.read().await;
    let listeners = data
        .get::<EventDrivenMessageContainer>()
        .ok_or(Error::Uninitialized)?;
    Ok(listeners.clone())
}
