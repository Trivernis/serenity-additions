use crate::error::Result;
use crate::events::RichEventHandler;
use crate::menu::traits::EventDrivenMessage;
use crate::menu::EventDrivenMessageContainer;
use dashmap::DashMap;
use serenity::client::ClientBuilder;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, MessageId};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

pub static SHORT_TIMEOUT: Duration = Duration::from_secs(5);
pub static MEDIUM_TIMEOUT: Duration = Duration::from_secs(20);
pub static LONG_TIMEOUT: Duration = Duration::from_secs(60);
pub static EXTRA_LONG_TIMEOUT: Duration = Duration::from_secs(600);

pub type BoxedEventDrivenMessage = Box<dyn EventDrivenMessage>;

pub struct BoxedMessage(pub BoxedEventDrivenMessage);

impl<T: EventDrivenMessage + 'static> From<Box<T>> for BoxedMessage {
    fn from(m: Box<T>) -> Self {
        Self(m as Box<dyn EventDrivenMessage>)
    }
}

impl Deref for BoxedMessage {
    type Target = BoxedEventDrivenMessage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BoxedMessage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct MessageHandle {
    pub channel_id: u64,
    pub message_id: u64,
}

impl MessageHandle {
    /// Creates a new message handle
    pub fn new(channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            message_id: message_id.0,
            channel_id: channel_id.0,
        }
    }

    /// Creates a new message handle from raw ids
    pub fn from_raw_ids(channel_id: u64, message_id: u64) -> Self {
        Self {
            message_id,
            channel_id,
        }
    }

    /// Returns the message object of the handle
    pub async fn get_message(&self, http: &Arc<Http>) -> Result<Message> {
        let msg = http.get_message(self.channel_id, self.message_id).await?;
        Ok(msg)
    }
}

pub trait RegisterAdditions {
    fn register_serenity_additions(self) -> Self;
    fn register_serenity_additions_with(self, rich_handler: RichEventHandler) -> Self;
}

impl<'a> RegisterAdditions for ClientBuilder<'a> {
    /// Registers the rich interactions configuration on the client
    fn register_serenity_additions(self) -> Self {
        self.register_serenity_additions_with(RichEventHandler::default())
    }

    /// Registers the rich interactions with a custom rich event handler
    fn register_serenity_additions_with(self, rich_handler: RichEventHandler) -> Self {
        self.type_map_insert::<EventDrivenMessageContainer>(Arc::new(DashMap::new()))
            .raw_event_handler(rich_handler)
    }
}
