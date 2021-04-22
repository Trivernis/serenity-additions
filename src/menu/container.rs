use crate::core::{BoxedEventDrivenMessage, MessageHandle};
use crate::Error;
use crate::Result;
use serenity::client::Context;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Container to store event driven messages in the serenity context data
pub struct EventDrivenMessageContainer;
pub type MessageRef = Arc<Mutex<BoxedEventDrivenMessage>>;
pub type EventDrivenMessagesRef = Arc<Mutex<HashMap<MessageHandle, MessageRef>>>;

impl TypeMapKey for EventDrivenMessageContainer {
    type Value = EventDrivenMessagesRef;
}

pub async fn get_listeners_from_context(ctx: &Context) -> Result<EventDrivenMessagesRef> {
    let data = ctx.data.read().await;
    let listeners = data
        .get::<EventDrivenMessageContainer>()
        .ok_or(Error::Uninitialized)?;
    log::trace!("Returning listener");
    Ok(listeners.clone())
}
