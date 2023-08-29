use std::env;
use tracing::error;

use poise::serenity_prelude::{self as serenity, CreateComponents, Webhook};

use crate::{
    dc_utils::CreateReplyAddon,
    utils::{self, IsacError, IsacInfo},
    Context, Error,
};

// TODO: might need to be moved to a file for consts
const OOPS: &str = "***Oops! Something went wrong!***
click the **Document** button to check the commands usage
If this error keep coming out, please join our support server to report it
";

const PREMIUM: &str =
    "Seems you haven't join our Patreon, or link your discord account on Patreon yet :(
If you do like ISAC, [take a look?]( https://www.patreon.com/ISAC_bot )";

pub async fn isac_err_handler(ctx: &Context<'_>, error: &IsacError) {
    match error {
        IsacError::Help(help) => {
            let msg = match help {
                utils::IsacHelp::LackOfArguments => {
                    "Click the button to check commands' usage and examples".to_string()
                }
            };
            isac_get_help(ctx, Some(msg.as_ref())).await;
        }
        IsacError::Info(info) => {
            let msg = match info {
                IsacInfo::UserNotLinked { user_name } => match user_name.as_ref() {
                    Some(user_name) => {
                        format!("**{user_name}** haven't linked to any wows account yet")
                    }
                    None => "You haven't linked your account yet.\nEnter `/link`".to_string(),
                },
                IsacInfo::TooShortIgn { ign } => {
                    format!("❌ At least 3 charactars for ign searching: `{ign}`")
                }
                IsacInfo::APIError { msg } => format!("❌ API error: `{msg}`"),
                IsacInfo::InvalidIgn { ign } => format!("❌ Invalid ign: `{ign}`"),
                IsacInfo::PlayerIgnNotFound { ign, region } => {
                    format!("Player: `{ign}` not found in `{region}`")
                }
                IsacInfo::PlayerHidden { ign } => {
                    format!("Player `{ign}`'s profile is hidden.")
                }
                IsacInfo::PlayerNoBattle { ign } => {
                    format!("Player `{ign}` hasn't played any battle.")
                }
                IsacInfo::GeneralError { msg } => msg.clone(),
                IsacInfo::InvalidClan { clan } => format!("❌ Invalid clan name: `{clan}`"),
                IsacInfo::ClanNotFound { clan, region } => {
                    format!("Clan: `{clan}` not found in `{region}`")
                }
                IsacInfo::ShipNotFound { ship_name } => format!("Warship: `{ship_name}` not found"),
                IsacInfo::PlayerNoBattleShip {
                    ign,
                    ship_name,
                    mode,
                } => {
                    format!(
                        "Player: `{ign}` hasn't played any battle in **{ship_name}** in **{}**",
                        mode.upper()
                    )
                }
                IsacInfo::AutoCompleteError => {
                    "❌ please select an option in the results!".to_string()
                }
                IsacInfo::ClanNoBattle { clan, season } => format!(
                    "**[{}]** ({}) did not participate in season **{}**",
                    clan.tag.replace("_", r"\_"),
                    clan.region,
                    season
                ),
                IsacInfo::NeedPremium { msg } => format!("{msg}\n{PREMIUM}"),
                IsacInfo::EmbedPermission => format!("❌ This error means ISAC don't have to permission to send embed here, please check the **Embed Links** in the permission setting, \nOr you can just re-invite ISAC in discord to let it grant the permission"),
            };
            let _r = ctx
                .send(|b| b.content(msg).reply(true).ephemeral(true))
                .await;
        }
        IsacError::Cancelled => (),
        IsacError::UnknownError(err) => {
            isac_err_logging(&ctx, err).await;
            isac_get_help(&ctx, None).await
        }
    };
}

// TODO: better error msg, python's tracback?
/// logging to the terminal and discord channel
pub async fn isac_err_logging(ctx: &Context<'_>, error: &Error) {
    let user: &str = ctx.author().name.as_ref();
    let user_id = ctx.author().id;
    let channel_id = ctx.channel_id();
    let guild = ctx.guild().map(|f| f.name).unwrap_or("PM".to_string());
    let input = ctx.invocation_string();
    let web_hook = Webhook::from_url(
        ctx,
        env::var("ERR_WEB_HOOK")
            .expect("Missing web hook url")
            .as_ref(),
    )
    .await
    .unwrap();
    let _r = web_hook
        .execute(ctx, false, |b| {
            b.content(format!(
                "``` ERROR \n[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} ``` ``` {error} ```"
            ))
        })
        .await;
    error!("[{input}] \n{user}, {user_id} \n{channel_id} \n{guild} \n{error}");
}

pub async fn isac_get_help(ctx: &Context<'_>, msg: Option<&str>) {
    let msg = match msg {
        Some(msg) => msg,
        None => OOPS,
    };
    let _r = ctx
        .send(|b| {
            b.content(msg)
                .set_components(help_view())
                .reply(true)
                .ephemeral(true)
        })
        .await;
}

fn help_view() -> CreateComponents {
    let mut view = CreateComponents::default();
    view.create_action_row(|r| {
        r.create_button(|b| {
            b.label("Document")
                .url("https://github.com/B-2U/ISAC")
                .style(serenity::ButtonStyle::Link)
        })
        .create_button(|b| {
            b.label("Support server")
                .url("https://discord.com/invite/z6sV6kEZGV")
                .style(serenity::ButtonStyle::Link)
        })
    });
    view
}
