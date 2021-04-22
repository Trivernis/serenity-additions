# Serenity Rich Interaction

This crate provides some types for rich interactions with serenity such as Menus and Ephemeral (self deleting) Messages.

## Usage

You have to register the module in the serenity client builder.
```rust
use serenity::client::Client;
use serenity_rich_interaction::RegisterRichInteractions;

#[tokio::main]
async fn get_client {
    // stuff
    let client = Client::builder("TOKEN").register_rich_interactions().await?;
    // stuff
}
```

## Menu

```rust
use serenity::builder::CreateMessage;
use serenity::client::Context;
use serenity::model::id::ChannelId;
use serenity_rich_interaction::menu::{MenuBuilder, Page};
use std::time::Duration;
use serenity_rich_interaction::Result;

pub async fn create_menu(
    ctx: &Context,
    channel_id: ChannelId,
) -> Result<()> {
    let mut message1 = CreateMessage::default();
    message1.content("Hello");
    let mut message2 = CreateMessage::default();
    message2.content("World");
    
    MenuBuilder::new_paginator()
        .timeout(Duration::from_secs(120))
        .add_page(Page::new_static(message1))
        .add_page(Page::new_static(message2))
        .show_help()
        .build(ctx, channel_id)
        .await?;

    Ok(())
}
```

## Ephemeral Message

```rust
use serenity_rich_interaction::core::SHORT_TIMEOUT;
use serenity_rich_interaction::ephemeral_message::EphemeralMessage;
use serenity_rich_interaction::Result;
use serenity::client::Context;
use serenity::model::id::ChannelId;

pub async fn create_ephemeral_message(ctx: &Context, channel_id: ChannelId) -> Result<()> {
    EphemeralMessage::create(&ctx.http, channel_id, SHORT_TIMEOUT, |m| {
        m.content("Hello World")
    }).await?;
    
    Ok(())
}

```


## License

MIT