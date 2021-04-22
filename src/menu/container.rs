use crate::core::{BoxedEventDrivenMessage, MessageHandle};
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
