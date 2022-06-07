use crate::core::MessageHandle;
use crate::error::{Error, Result};
use crate::menu::controls::{close_menu, next_page, previous_page, toggle_help};
use crate::menu::traits::EventDrivenMessage;
use crate::menu::typedata::HelpActiveContainer;
use crate::menu::{get_listeners_from_context, EventDrivenMessagesRef, Page};
use futures::FutureExt;
use serenity::async_trait;
use serenity::client::Context;
use serenity::http::Http;
use serenity::model::channel::{Message, Reaction, ReactionType};
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::{TypeMap, TypeMapKey};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

pub static NEXT_PAGE_EMOJI: &str = "➡️";
pub static PREVIOUS_PAGE_EMOJI: &str = "⬅️";
pub static CLOSE_MENU_EMOJI: &str = "❌";
pub static HELP_EMOJI: &str = "❔";

pub type ControlActionResult<'b> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'b>>;

pub type ControlActionArc = Arc<
    dyn for<'b> Fn(&'b Context, &'b mut Menu<'_>, Reaction) -> ControlActionResult<'b>
        + Send
        + Sync,
>;

#[derive(Clone)]
pub struct ActionContainer {
    inner: ControlActionArc,
    position: isize,
}

impl ActionContainer {
    /// Creates a new control action
    pub fn new<F: 'static>(position: isize, callback: F) -> Self
    where
        F: for<'b> Fn(&'b Context, &'b mut Menu<'_>, Reaction) -> ControlActionResult<'b>
            + Send
            + Sync,
    {
        Self {
            inner: Arc::new(callback),
            position,
        }
    }

    /// Runs the action
    pub async fn run(&self, ctx: &Context, menu: &mut Menu<'_>, reaction: Reaction) -> Result<()> {
        self.inner.clone()(ctx, menu, reaction).await?;
        Ok(())
    }

    /// Returns the position of the action
    pub fn position(&self) -> isize {
        self.position
    }
}

/// A menu message
pub struct Menu<'a> {
    pub message: Arc<RwLock<MessageHandle>>,
    pub pages: Vec<Page<'a>>,
    pub current_page: usize,
    pub(crate) controls: HashMap<String, ActionContainer>,
    pub timeout: Instant,
    pub sticky: bool,
    pub data: TypeMap,
    pub(crate) help_entries: HashMap<String, String>,
    owner: Option<UserId>,
    closed: bool,
    listeners: EventDrivenMessagesRef,
}

impl<'a> Menu<'a> {
    /// Returns the current page of the menu
    pub fn get_current_page(&self) -> Result<&Page<'a>> {
        self.pages
            .get(self.current_page)
            .ok_or(Error::PageNotFound(self.current_page))
    }

    /// Removes all reactions from the menu
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) async fn close(&mut self, http: &Http) -> Result<()> {
        let handle = self.message.read().await;
        http.delete_message_reactions(handle.channel_id, handle.message_id)
            .await?;
        self.closed = true;
        Ok(())
    }

    /// Returns the message of the menu
    pub async fn get_message(&self, http: &Http) -> Result<Message> {
        let handle = self.message.read().await;
        let msg = http
            .get_message(handle.channel_id, handle.message_id)
            .await?;

        Ok(msg)
    }

    /// Recreates the message completely
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn recreate(&self, http: &Http) -> Result<()> {
        let old_handle = self.get_handle().await;
        let current_page = self.get_current_page()?.get().await?;

        let message = http
            .send_message(
                old_handle.channel_id,
                &serde_json::to_value(current_page.0).unwrap(),
            )
            .await?;

        for control in &self.controls {
            http.create_reaction(
                message.channel_id.0,
                message.id.0,
                &ReactionType::Unicode(control.0.clone()),
            )
            .await?;
        }

        let new_handle = {
            let mut handle = self.message.write().await;
            handle.message_id = message.id.0;
            (*handle).clone()
        };
        {
            tracing::debug!("Changing key of message");
            let menu = self.listeners.remove(&old_handle).unwrap();
            tracing::debug!("Inserting new key");
            self.listeners.insert(new_handle, menu.1);
        }
        tracing::debug!("Deleting original message");
        http.delete_message(old_handle.channel_id, old_handle.message_id)
            .await?;

        Ok(())
    }

    /// Returns the handle of the menus message
    /// Locking behaviour: May deadlock when already holding a lock to [Self::messages]
    async fn get_handle(&self) -> MessageHandle {
        let handle = self.message.read().await;
        (*handle).clone()
    }
}

#[async_trait]
impl<'a> EventDrivenMessage for Menu<'a> {
    fn is_frozen(&self) -> bool {
        self.closed
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn update(&mut self, http: &Http) -> Result<()> {
        tracing::trace!("Checking for menu timeout");

        if Instant::now() >= self.timeout {
            tracing::debug!("Menu timout reached. Closing menu.");
            self.close(http).await?;
        } else if self.sticky {
            tracing::debug!("Message is sticky. Checking for new messages in channel...");

            let handle = self.get_handle().await;
            let channel_id = ChannelId(handle.channel_id);
            let messages = channel_id
                .messages(http, |p| p.after(handle.message_id).limit(1))
                .await?;
            if messages.len() > 0 {
                tracing::debug!("New messages in channel. Recreating...");
                self.recreate(http).await?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn on_reaction_add(&mut self, ctx: &Context, reaction: Reaction) -> Result<()> {
        let current_user = ctx.http.get_current_user().await?;
        let reaction_user_id = reaction.user_id.ok_or_else(|| Error::NoCache)?;

        if reaction_user_id.0 == current_user.id.0 {
            tracing::debug!("Reaction is from current user.");
            return Ok(());
        }
        let emoji_string = reaction.emoji.as_data();

        tracing::debug!("Deleting user reaction.");
        reaction.delete(ctx).await?;

        if let Some(owner) = self.owner {
            if owner != reaction_user_id {
                tracing::debug!(
                    "Menu has an owner and the reaction is not from the owner of the menu"
                );
                return Ok(());
            }
        }
        if let Some(control) = self.controls.get(&emoji_string).cloned() {
            tracing::debug!("Running control");
            control.run(ctx, self, reaction).await?;
        }

        Ok(())
    }
}

/// A builder for messages
pub struct MenuBuilder {
    pages: Vec<Page<'static>>,
    current_page: usize,
    controls: HashMap<String, ActionContainer>,
    timeout: Duration,
    sticky: bool,
    data: TypeMap,
    help_entries: HashMap<String, String>,
    owner: Option<UserId>,
}

impl Default for MenuBuilder {
    fn default() -> Self {
        Self {
            pages: vec![],
            current_page: 0,
            controls: HashMap::new(),
            timeout: Duration::from_secs(60),
            sticky: false,
            data: TypeMap::new(),
            help_entries: HashMap::new(),
            owner: None,
        }
    }
}

impl MenuBuilder {
    /// Creates a new pagination menu
    #[tracing::instrument(level = "debug", skip_all)]
    pub fn new_paginator() -> Self {
        let mut controls = HashMap::new();
        let mut help_entries = HashMap::new();
        controls.insert(
            PREVIOUS_PAGE_EMOJI.to_string(),
            ActionContainer::new(0, |c, m, r| previous_page(c, m, r).boxed()),
        );
        help_entries.insert(
            PREVIOUS_PAGE_EMOJI.to_string(),
            "Displays the previous page".to_string(),
        );
        controls.insert(
            CLOSE_MENU_EMOJI.to_string(),
            ActionContainer::new(1, |c, m, r| close_menu(c, m, r).boxed()),
        );
        help_entries.insert(
            CLOSE_MENU_EMOJI.to_string(),
            "Closes the menu buttons".to_string(),
        );
        controls.insert(
            NEXT_PAGE_EMOJI.to_string(),
            ActionContainer::new(2, |c, m, r| next_page(c, m, r).boxed()),
        );
        help_entries.insert(
            NEXT_PAGE_EMOJI.to_string(),
            "Displays the next page".to_string(),
        );

        Self {
            controls,
            help_entries,
            ..Default::default()
        }
    }

    /// Adds a page to the message builder
    pub fn add_page(mut self, page: Page<'static>) -> Self {
        self.pages.push(page);

        self
    }

    /// Adds multiple pages to the message
    pub fn add_pages<I>(mut self, pages: I) -> Self
    where
        I: IntoIterator<Item = Page<'static>>,
    {
        let mut pages = pages.into_iter().collect();
        self.pages.append(&mut pages);

        self
    }

    /// Adds a single control to the message
    pub fn add_control<S, F: 'static>(mut self, position: isize, emoji: S, action: F) -> Self
    where
        S: ToString,
        F: for<'b> Fn(&'b Context, &'b mut Menu<'_>, Reaction) -> ControlActionResult<'b>
            + Send
            + Sync,
    {
        self.controls
            .insert(emoji.to_string(), ActionContainer::new(position, action));

        self
    }

    /// Adds a single control to the message
    pub fn add_controls<S, I>(mut self, controls: I) -> Self
    where
        S: ToString,
        I: IntoIterator<Item = (isize, S, ControlActionArc)>,
    {
        for (position, emoji, action) in controls {
            self.controls.insert(
                emoji.to_string(),
                ActionContainer {
                    position,
                    inner: action,
                },
            );
        }

        self
    }

    /// Sets the timeout for the message
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;

        self
    }

    /// Sets the start page of the message
    pub fn start_page(mut self, page: usize) -> Self {
        self.current_page = page;

        self
    }

    /// If the message should be sticky and always be
    /// the last one in the channel
    pub fn sticky(mut self, value: bool) -> Self {
        self.sticky = value;

        self
    }

    /// Adds data to the menu typemap
    pub fn add_data<T>(mut self, value: T::Value) -> Self
    where
        T: TypeMapKey,
    {
        self.data.insert::<T>(value);

        self
    }

    /// Adds a help entry
    pub fn add_help<S: ToString>(mut self, button: S, help: S) -> Self {
        self.help_entries
            .insert(button.to_string(), help.to_string());

        self
    }

    /// Turns showing help for buttons on
    pub fn show_help(self) -> Self {
        self.add_control(100, HELP_EMOJI, |c, m, r| Box::pin(toggle_help(c, m, r)))
            .add_data::<HelpActiveContainer>(Arc::new(AtomicBool::new(false)))
    }

    /// Sets the owner of the menu
    /// if it's set only the owner can interact with the menu
    pub fn owner(mut self, user_id: UserId) -> Self {
        self.owner = Some(user_id);

        self
    }

    /// builds the menu
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn build(
        self,
        ctx: &Context,
        channel_id: ChannelId,
    ) -> Result<Arc<RwLock<MessageHandle>>> {
        let mut current_page = self
            .pages
            .get(self.current_page)
            .ok_or(Error::PageNotFound(self.current_page))?
            .clone()
            .get()
            .await?;

        let message = channel_id.send_message(ctx, |_| &mut current_page).await?;

        tracing::debug!("Sorting controls...");
        let mut controls = self
            .controls
            .clone()
            .into_iter()
            .collect::<Vec<(String, ActionContainer)>>();
        controls.sort_by_key(|(_, a)| a.position);

        tracing::debug!("Creating menu...");
        let message_handle = MessageHandle::new(message.channel_id, message.id);
        let handle_lock = Arc::new(RwLock::new(message_handle));
        let listeners = get_listeners_from_context(ctx).await?;

        let menu = Menu {
            message: Arc::clone(&handle_lock),
            pages: self.pages,
            current_page: self.current_page,
            controls: self.controls,
            timeout: Instant::now() + self.timeout,
            closed: false,
            listeners: Arc::clone(&listeners),
            sticky: self.sticky,
            data: self.data,
            help_entries: self.help_entries,
            owner: self.owner,
        };

        tracing::debug!("Storing menu to listeners...");
        listeners.insert(message_handle, Arc::new(Mutex::new(Box::new(menu).into())));

        tracing::debug!("Adding controls...");
        for (emoji, _) in controls {
            message
                .react(ctx, ReactionType::Unicode(emoji.clone()))
                .await?;
        }

        Ok(handle_lock)
    }
}
