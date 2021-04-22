use crate::events::event_callbacks;
use crate::Result;
use serenity::async_trait;
use serenity::client::{Context, RawEventHandler};
use serenity::model::event;
use serenity::model::event::Event;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct EventCallback<T> {
    inner: Arc<
        dyn for<'a> Fn(
                &'a Context,
                &'a T,
            ) -> Pin<Box<(dyn Future<Output = Result<()>> + Send + 'a)>>
            + Send
            + Sync,
    >,
}

impl<T> EventCallback<T> {
    pub async fn run(&self, ctx: &Context, arg: &T) -> Result<()> {
        self.inner.clone()(ctx, arg).await?;
        Ok(())
    }
}

/// A handler for raw serenity events
/// ```
/// use serenity_rich_interaction::events::RichEventHandler;
/// use serenity::model::event;
/// use serenity::client::Client;
///
/// let mut handler = RichEventHandler::default();
/// handler.add_event(|ctx, e: &event::ReadyEvent| Box::pin(async move {
///     println!("Ready event received");
///     Ok(())
/// }));
/// let client = Client::builder("TOKEN").raw_event_handler(handler).await?;
/// // ...
/// ```
pub struct RichEventHandler {
    callbacks: HashMap<TypeId, Vec<Arc<dyn Any + Send + Sync>>>,
}

impl RichEventHandler {
    /// Handles a generic event
    async fn handle_event<T: 'static + Send + Sync>(&self, ctx: Context, value: T) {
        let callbacks = self.callbacks.clone();

        tokio::spawn(async move {
            let value = value;
            if let Some(callbacks) = callbacks.get(&TypeId::of::<T>()) {
                for callback in callbacks {
                    if let Some(cb) = callback.downcast_ref::<EventCallback<T>>() {
                        if let Err(e) = cb.run(&ctx, &value).await {
                            log::error!("Error in event callback: {:?}", e);
                        }
                    }
                }
            }
        });
    }

    pub fn add_event<T: 'static, F: 'static>(&mut self, cb: F) -> &mut Self
    where
        F: for<'a> Fn(
                &'a Context,
                &'a T,
            ) -> Pin<Box<(dyn Future<Output = Result<()>> + Send + 'a)>>
            + Send
            + Sync,
    {
        let type_id = TypeId::of::<T>();
        let callbacks = if let Some(cbs) = self.callbacks.get_mut(&type_id) {
            cbs
        } else {
            self.callbacks.insert(type_id, Vec::new());
            self.callbacks.get_mut(&type_id).unwrap()
        };
        callbacks.push(Arc::new(EventCallback {
            inner: Arc::new(cb),
        }));

        self
    }
}

impl Default for RichEventHandler {
    fn default() -> Self {
        let mut handler = Self {
            callbacks: Default::default(),
        };
        handler
            .add_event(|ctx, _: &event::ReadyEvent| {
                Box::pin(event_callbacks::start_update_loop(ctx))
            })
            .add_event(|ctx, e: &event::ReactionAddEvent| {
                Box::pin(event_callbacks::handle_reaction_add(ctx, &e.reaction))
            })
            .add_event(|ctx, e: &event::ReactionRemoveEvent| {
                Box::pin(event_callbacks::handle_reaction_remove(ctx, &e.reaction))
            })
            .add_event(|ctx, e: &event::MessageDeleteEvent| {
                Box::pin(event_callbacks::handle_message_delete(
                    ctx,
                    e.channel_id,
                    e.message_id,
                ))
            })
            .add_event(|ctx, e: &event::MessageDeleteBulkEvent| {
                Box::pin(event_callbacks::handle_message_delete_bulk(
                    ctx,
                    e.channel_id,
                    &e.ids,
                ))
            });

        handler
    }
}

#[async_trait]
impl RawEventHandler for RichEventHandler {
    async fn raw_event(&self, ctx: Context, event: Event) {
        match event {
            Event::ChannelCreate(e) => self.handle_event(ctx, e).await,
            Event::ChannelDelete(e) => self.handle_event(ctx, e).await,
            Event::ChannelPinsUpdate(e) => self.handle_event(ctx, e).await,
            Event::ChannelUpdate(e) => self.handle_event(ctx, e).await,
            Event::GuildBanAdd(e) => self.handle_event(ctx, e).await,
            Event::GuildBanRemove(e) => self.handle_event(ctx, e).await,
            Event::GuildCreate(e) => self.handle_event(ctx, e).await,
            Event::GuildDelete(e) => self.handle_event(ctx, e).await,
            Event::GuildEmojisUpdate(e) => self.handle_event(ctx, e).await,
            Event::GuildIntegrationsUpdate(e) => self.handle_event(ctx, e).await,
            Event::GuildMemberAdd(e) => self.handle_event(ctx, e).await,
            Event::GuildMemberRemove(e) => self.handle_event(ctx, e).await,
            Event::GuildMemberUpdate(e) => self.handle_event(ctx, e).await,
            Event::GuildMembersChunk(e) => self.handle_event(ctx, e).await,
            Event::GuildRoleCreate(e) => self.handle_event(ctx, e).await,
            Event::GuildRoleDelete(e) => self.handle_event(ctx, e).await,
            Event::GuildRoleUpdate(e) => self.handle_event(ctx, e).await,
            Event::GuildUnavailable(e) => self.handle_event(ctx, e).await,
            Event::GuildUpdate(e) => self.handle_event(ctx, e).await,
            Event::InviteCreate(e) => self.handle_event(ctx, e).await,
            Event::InviteDelete(e) => self.handle_event(ctx, e).await,
            Event::MessageCreate(e) => self.handle_event(ctx, e).await,
            Event::MessageDelete(e) => self.handle_event(ctx, e).await,
            Event::MessageDeleteBulk(e) => self.handle_event(ctx, e).await,
            Event::MessageUpdate(e) => self.handle_event(ctx, e).await,
            Event::PresenceUpdate(e) => self.handle_event(ctx, e).await,
            Event::PresencesReplace(e) => self.handle_event(ctx, e).await,
            Event::ReactionAdd(e) => self.handle_event(ctx, e).await,
            Event::ReactionRemove(e) => self.handle_event(ctx, e).await,
            Event::ReactionRemoveAll(e) => self.handle_event(ctx, e).await,
            Event::Ready(e) => self.handle_event(ctx, e).await,
            Event::Resumed(e) => self.handle_event(ctx, e).await,
            Event::TypingStart(e) => self.handle_event(ctx, e).await,
            Event::UserUpdate(e) => self.handle_event(ctx, e).await,
            Event::VoiceStateUpdate(e) => self.handle_event(ctx, e).await,
            Event::VoiceServerUpdate(e) => self.handle_event(ctx, e).await,
            Event::WebhookUpdate(e) => self.handle_event(ctx, e).await,
            Event::Unknown(e) => self.handle_event(ctx, e).await,
            _ => {}
        }
    }
}
