use crate::core::MessageHandle;
use crate::menu::{get_listeners_from_context, MessageRef};
use crate::Result;
use serenity::client::Context;
use serenity::model::channel::Reaction;
use serenity::model::id::{ChannelId, MessageId};
use std::sync::Arc;
use tokio::time::Duration;

static UPDATE_INTERVAL_SECS: u64 = 5;

/// Starts the loop to handle message updates
pub async fn start_update_loop(ctx: &Context) -> Result<()> {
    let event_messages = get_listeners_from_context(ctx)
        .await
        .expect("Failed to get event message container");
    let http = Arc::clone(&ctx.http);

    tokio::task::spawn(async move {
        loop {
            {
                log::trace!("Locking listener from update loop.");
                let messages = {
                    let msgs_lock = event_messages.lock().await;

                    msgs_lock
                        .iter()
                        .map(|(k, v)| (*k, v.clone()))
                        .collect::<Vec<(MessageHandle, MessageRef)>>()
                };
                log::trace!("Listener locked.");
                let mut frozen_messages = Vec::new();

                for (key, msg) in messages {
                    let mut msg = msg.lock().await;
                    if let Err(e) = msg.update(&http).await {
                        log::error!("Failed to update message: {:?}", e);
                    }
                    if msg.is_frozen() {
                        frozen_messages.push(key);
                    }
                }
                {
                    let mut msgs_lock = event_messages.lock().await;
                    for key in frozen_messages {
                        msgs_lock.remove(&key);
                    }
                }
            }
            log::trace!("Listener unlocked");
            tokio::time::sleep(Duration::from_secs(UPDATE_INTERVAL_SECS)).await;
        }
    });

    Ok(())
}

/// To be fired from the serenity handler when a message was deleted
pub async fn handle_message_delete(
    ctx: &Context,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<()> {
    let mut affected_messages = Vec::new();
    {
        let listeners = get_listeners_from_context(ctx).await?;
        log::trace!("Locking listener from handle_message_delete.");
        let mut listeners_lock = listeners.lock().await;
        log::trace!("Listener locked.");

        let handle = MessageHandle::new(channel_id, message_id);
        if let Some(msg) = listeners_lock.get(&handle) {
            affected_messages.push(Arc::clone(msg));
            listeners_lock.remove(&handle);
        }
    }
    log::trace!("Listener unlocked");
    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_deleted(ctx).await?;
    }

    Ok(())
}

/// To be fired from the serenity handler when multiple messages were deleted
pub async fn handle_message_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    message_ids: &Vec<MessageId>,
) -> Result<()> {
    let mut affected_messages = Vec::new();
    {
        let listeners = get_listeners_from_context(ctx).await?;
        log::trace!("Locking listener from handle_message_delete_bulk.");
        let mut listeners_lock = listeners.lock().await;
        log::trace!("Listener locked.");

        for message_id in message_ids {
            let handle = MessageHandle::new(channel_id, *message_id);
            if let Some(msg) = listeners_lock.get_mut(&handle) {
                affected_messages.push(Arc::clone(msg));
                listeners_lock.remove(&handle);
            }
        }
    }
    log::trace!("Listener unlocked");
    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_deleted(ctx).await?;
    }

    Ok(())
}

/// Fired when a reaction was added to a message
pub async fn handle_reaction_add(ctx: &Context, reaction: &Reaction) -> Result<()> {
    let mut affected_messages = Vec::new();
    {
        let listeners = get_listeners_from_context(ctx).await?;
        log::trace!("Locking listener from handle_reaction_add.");
        let mut listeners_lock = listeners.lock().await;
        log::trace!("Listener locked.");

        let handle = MessageHandle::new(reaction.channel_id, reaction.message_id);

        if let Some(msg) = listeners_lock.get_mut(&handle) {
            affected_messages.push(Arc::clone(&msg));
        }
    }
    log::trace!("Listener unlocked");
    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_reaction_add(ctx, reaction.clone()).await?;
    }

    Ok(())
}

/// Fired when a reaction was added to a message
pub async fn handle_reaction_remove(ctx: &Context, reaction: &Reaction) -> Result<()> {
    let mut affected_messages = Vec::new();
    {
        let listeners = get_listeners_from_context(ctx).await?;
        log::trace!("Locking listener from handle_reaction_remove.");
        let mut listeners_lock = listeners.lock().await;
        log::trace!("Listener locked.");

        let handle = MessageHandle::new(reaction.channel_id, reaction.message_id);

        if let Some(msg) = listeners_lock.get_mut(&handle) {
            affected_messages.push(Arc::clone(&msg));
        }
    }
    log::trace!("Listener unlocked");
    for msg in affected_messages {
        let mut msg = msg.lock().await;
        msg.on_reaction_remove(ctx, reaction.clone()).await?;
    }

    Ok(())
}
