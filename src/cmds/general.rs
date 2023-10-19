use poise::serenity_prelude::CreateEmbed;

use crate::dc_utils::EasyEmbed;
use crate::{Context, Error};

/// The link for inviting ISAC
#[poise::command(prefix_command, slash_command, discard_spare_arguments)]
pub async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    const INVITE_MSG: &str = "[Click here](https://discord.com/oauth2/authorize?client_id=961882964034203648&permissions=0&scope=bot%20applications.commands) to invite ISAC";

    ctx.send(|b| b.embed(|e| CreateEmbed::default_from(e).description(INVITE_MSG)))
        .await?;
    Ok(())
}

/// Show the help urls
#[poise::command(prefix_command, slash_command, discard_spare_arguments)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    const HELP_MSG: &str = "Support server: https://discord.gg/nk7PSYrFWu \nCommands list: https://github.com/B-2U/ISAC";
    ctx.reply(HELP_MSG).await?;
    Ok(())
}
