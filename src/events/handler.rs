use crate::events::event_callbacks;
use crate::Result;
use futures::future;
use serenity::async_trait;
use serenity::client::{Context, RawEventHandler};
use serenity::model::event;
use serenity::model::event::Event;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

macro_rules! handle_events {
    (
        match $evt:ident {
            $($variant:pat) | + => $handle_call:expr,
        }
    ) => {
        match $evt {
            $($variant => $handle_call),+,
            _ => {},
        }
    }
}

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
/// use serenity_additions::events::RichEventHandler;
/// use serenity::model::event;
/// use serenity::client::Client;
/// use serenity::prelude::GatewayIntents;
/// use serenity_additions::RegisterAdditions;
/// # async fn a() -> serenity_additions::Result<()> {
///
/// let mut handler = RichEventHandler::default();
/// handler.add_event(|ctx, e: &event::ReadyEvent| Box::pin(async move {
///     println!("Ready event received");
///     Ok(())
/// }));
/// let client = Client::builder("TOKEN", GatewayIntents::default()).register_serenity_additions_with(handler).await?;
/// // ...
/// # unimplemented!()
/// # }
/// ```
pub struct RichEventHandler {
    callbacks: HashMap<TypeId, Vec<Arc<dyn Any + Send + Sync>>>,
}

impl RichEventHandler {
    /// Handles a generic event
    #[tracing::instrument(level = "debug", skip_all)]
    async fn handle_event<T: 'static + Send + Sync>(&self, ctx: Context, value: T) {
        let value = value;
        if let Some(callbacks) = self.callbacks.get(&TypeId::of::<T>()) {
            let futures = callbacks
                .iter()
                .filter_map(|cb| cb.downcast_ref::<EventCallback<T>>())
                .map(|cb| cb.run(&ctx, &value));
            future::join_all(futures)
                .await
                .into_iter()
                .filter_map(Result::err)
                .for_each(|e| tracing::error!("Error in event callback: {:?}", e));
        }
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
        handle_events!(match event {
            Event::ChannelCreate(e)
            | Event::ChannelDelete(e)
            | Event::ChannelPinsUpdate(e)
            | Event::ChannelUpdate(e)
            | Event::GuildBanAdd(e)
            | Event::GuildBanRemove(e)
            | Event::GuildCreate(e)
            | Event::GuildDelete(e)
            | Event::GuildEmojisUpdate(e)
            | Event::GuildIntegrationsUpdate(e)
            | Event::GuildMemberAdd(e)
            | Event::GuildMemberRemove(e)
            | Event::GuildMemberUpdate(e)
            | Event::GuildMembersChunk(e)
            | Event::GuildRoleCreate(e)
            | Event::GuildRoleDelete(e)
            | Event::GuildRoleUpdate(e)
            | Event::GuildUnavailable(e)
            | Event::GuildUpdate(e)
            | Event::InviteCreate(e)
            | Event::InviteDelete(e)
            | Event::MessageCreate(e)
            | Event::MessageDelete(e)
            | Event::MessageDeleteBulk(e)
            | Event::MessageUpdate(e)
            | Event::PresenceUpdate(e)
            | Event::PresencesReplace(e)
            | Event::ReactionAdd(e)
            | Event::ReactionRemove(e)
            | Event::ReactionRemoveAll(e)
            | Event::Ready(e)
            | Event::Resumed(e)
            | Event::TypingStart(e)
            | Event::UserUpdate(e)
            | Event::VoiceStateUpdate(e)
            | Event::VoiceServerUpdate(e)
            | Event::WebhookUpdate(e)
            | Event::Unknown(e)
            | Event::InteractionCreate(e)
            | Event::IntegrationCreate(e)
            | Event::IntegrationUpdate(e)
            | Event::IntegrationDelete(e)
            | Event::StageInstanceCreate(e)
            | Event::StageInstanceUpdate(e)
            | Event::StageInstanceDelete(e)
            | Event::ThreadCreate(e)
            | Event::ThreadUpdate(e)
            | Event::ThreadDelete(e)
            | Event::ThreadListSync(e)
            | Event::ThreadMemberUpdate(e)
            | Event::ThreadMembersUpdate(e) => self.handle_event(ctx, e).await,
        });
    }
}
