use crate::core::MessageHandle;
use crate::menu::get_listeners_from_context;
use crate::Result;
use serenity::client::Context;
use serenity::model::channel::Reaction;
use serenity::model::id::{ChannelId, MessageId};
use std::sync::Arc;
use tokio::time::Duration;

static UPDATE_INTERVAL_SECS: u64 = 5;

/// Starts the loop to handle message updates
#[tracing::instrument(level = "debug", skip(ctx))]
pub async fn start_update_loop(ctx: &Context) -> Result<()> {
    let event_messages = get_listeners_from_context(ctx)
        .await
        .expect("Failed to get event message container");
    let http = Arc::clone(&ctx.http);

    tokio::task::spawn(async move {
        loop {
            {
                tracing::trace!("Updating messages...");
                let mut frozen_messages = Vec::new();

                for (key, value) in event_messages
                    .iter()
                    .map(|e| (e.key().clone(), e.value().clone()))
                {
                    let mut msg = value.lock().await;
                    if let Err(e) = msg.update(&http).await {
                        tracing::error!("Failed to update message: {:?}", e);
                    }
                    if msg.is_frozen() {
                        frozen_messages.push(key);
                    }
                }
                for key in frozen_messages {
                    event_messages.remove(&key);
                }
                tracing::trace!("Messages updated");
            }
            tokio::time::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS)).await;
        }
    });

    Ok(())
}

/// To be fired from the serenity handler when a message was deleted
#[tracing::instrument(level = "debug", skip(ctx))]
pub async fn handle_message_delete(
    ctx: &Context,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<()> {
    let mut affected_messages = Vec::new();
    let listeners = get_listeners_from_context(ctx).await?;
    let handle = MessageHandle::new(channel_id, message_id);

    if let Some(msg) = listeners.get(&handle) {
        affected_messages.push(msg.value().clone());
        listeners.remove(&handle);
    }

    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_deleted(ctx).await?;
    }

    Ok(())
}

/// To be fired from the serenity handler when multiple messages were deleted
#[tracing::instrument(level = "debug", skip(ctx))]
pub async fn handle_message_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    message_ids: &Vec<MessageId>,
) -> Result<()> {
    let mut affected_messages = Vec::new();

    let listeners = get_listeners_from_context(ctx).await?;

    for message_id in message_ids {
        let handle = MessageHandle::new(channel_id, *message_id);
        if let Some(msg) = listeners.get(&handle) {
            affected_messages.push(msg.value().clone());
            listeners.remove(&handle);
        }
    }

    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_deleted(ctx).await?;
    }

    Ok(())
}

/// Fired when a reaction was added to a message
#[tracing::instrument(level = "debug", skip(ctx))]
pub async fn handle_reaction_add(ctx: &Context, reaction: &Reaction) -> Result<()> {
    let listeners = get_listeners_from_context(ctx).await?;
    let handle = MessageHandle::new(reaction.channel_id, reaction.message_id);

    let mut affected_messages = Vec::new();
    if let Some(msg) = listeners.get(&handle) {
        affected_messages.push(msg.value().clone());
    }

    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_reaction_add(ctx, reaction.clone()).await?;
    }

    Ok(())
}

/// Fired when a reaction was added to a message
#[tracing::instrument(level = "debug", skip(ctx))]
pub async fn handle_reaction_remove(ctx: &Context, reaction: &Reaction) -> Result<()> {
    let listeners = get_listeners_from_context(ctx).await?;
    let handle = MessageHandle::new(reaction.channel_id, reaction.message_id);

    let mut affected_messages = Vec::new();
    if let Some(msg) = listeners.get(&handle) {
        affected_messages.push(msg.value().clone());
    }

    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_reaction_remove(ctx, reaction.clone()).await?;
    }

    Ok(())
}
