use std::collections::HashMap;

use crate::{dc_utils::ContextAddon, utils::LoadFromJson, Context, Error};
use poise::{
    futures_util::StreamExt,
    serenity_prelude::{
        self as serenity, ArgumentConvert, CacheHttp, Channel, CreateActionRow, CreateButton,
        ReactionType,
    },
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone)]
struct TestView {
    btn_1: CreateButton,
    btn_2: CreateButton,
    btn_3: CreateButton,
}
impl Default for TestView {
    fn default() -> Self {
        TestView {
            btn_1: CreateButton::default()
                .label("1")
                .custom_id("owner_test_1")
                .to_owned(),
            btn_2: CreateButton::default()
                .label("2")
                .custom_id("owner_test_2")
                .to_owned(),
            btn_3: CreateButton::default()
                .label("3")
                .custom_id("owner_test_3")
                .to_owned(),
        }
    }
}
impl TestView {
    fn build(&self) -> CreateActionRow {
        CreateActionRow::default()
            .add_button(self.btn_1.clone())
            .add_button(self.btn_2.clone())
            .add_button(self.btn_3.clone())
            .to_owned()
    }
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn test(ctx: Context<'_>, #[rest] args: Option<String>) -> Result<(), Error> {
    let _args: Vec<String> = args
        .unwrap_or_default()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut view = TestView::default();
    let inter_msg = ctx
        .send(|b| {
            b.components(|f| f.set_action_row(view.build()))
                .ephemeral(true)
        })
        .await?;
    while let Some(interaction) = inter_msg
        .message()
        .await?
        .await_component_interactions(ctx)
        .build()
        .next()
        .await
    {
        match interaction.data.custom_id.as_str() {
            "owner_test_1" => {
                view.btn_1.disabled(true);
                view.btn_2.disabled(false);
                view.btn_3.disabled(false);
            }
            "owner_test_2" => {
                view.btn_1.disabled(false);
                view.btn_2.disabled(true);
                view.btn_3.disabled(false);
            }
            "owner_test_3" => {
                view.btn_1.disabled(false);
                view.btn_2.disabled(false);
                view.btn_3.disabled(true);
            }
            _ => (),
        }
        interaction
            .create_interaction_response(ctx, |b| {
                b.kind(serenity::InteractionResponseType::UpdateMessage);
                b.interaction_response_data(|a| a.components(|c| c.set_action_row(view.build())))
            })
            .await?;
    }
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn guilds(ctx: Context<'_>) -> Result<(), Error> {
    let _cache = ctx.cache().unwrap();
    ctx.reply(_cache.guilds().len().to_string()).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn users(ctx: Context<'_>) -> Result<(), Error> {
    let players: HashMap<_, _> = Linked::load_json("./user_data/linked.json").await?.into();
    let _a = ctx.reply(players.len().to_string()).await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn send(ctx: Context<'_>, channel_id: String, #[rest] msg: String) -> Result<(), Error> {
    let Context::Prefix(prefix_ctx) = ctx else {
        Err("not a prefix context!")?
    };
    let channel = Channel::convert(
        ctx.serenity_context(),
        ctx.guild_id(),
        Some(ctx.channel_id()),
        &channel_id,
    )
    .await;
    let result_emoji = match channel {
        Ok(channel) => {
            channel.id().say(ctx, msg).await?;
            "✅".to_string()
        }
        Err(_) => "❌".to_string(),
    };
    prefix_ctx
        .msg
        .react(ctx, ReactionType::Unicode(result_emoji))
        .await?;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct Linked(pub HashMap<String, Value>);

impl From<Linked> for HashMap<String, Value> {
    fn from(value: Linked) -> Self {
        value.0
    }
}
