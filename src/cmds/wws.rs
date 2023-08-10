use std::{borrow::Cow, collections::HashMap, time::Duration};

use poise::serenity_prelude::{AttachmentType, ButtonStyle, CreateActionRow, CreateButton};

use crate::{
    dc_utils::{Args, ContextAddon, InteractionAddon},
    utils::structs::{
        template_data::{OverallData, OverallDataClass, OverallDataDiv, OverallDataTier},
        Mode, PartialPlayer, ShipClass, ShipTier, Statistic,
    },
    Context, Error,
};

#[poise::command(prefix_command)]
pub async fn wws(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    let mut args = args.unwrap_or_default();
    let typing = ctx.typing().await;

    let partial_player = args.parse_user(&ctx).await?;
    let player = partial_player.get_player(&ctx).await?;
    let clan = player.clan(&ctx).await?;
    if args.is_empty() {
        // TODO 分類過程優化?
        // wws
        let ships = player.all_ships(&ctx).await?;
        let div = OverallDataDiv::new(
            ships.to_statistic(&ctx.data().expected_js, Mode::Pvp),
            ships.to_statistic(&ctx.data().expected_js, Mode::Solo),
            ships.to_statistic(&ctx.data().expected_js, Mode::Div2),
            ships.to_statistic(&ctx.data().expected_js, Mode::Div3),
        );
        let class: OverallDataClass = ships
            .clone()
            .sort_class(&ctx)
            .into_iter()
            .map(|(class, ships)| {
                (
                    class,
                    ships.to_statistic(&ctx.data().expected_js, Mode::Pvp),
                )
            })
            .collect::<HashMap<ShipClass, Statistic>>()
            .into();
        let tier: OverallDataTier = ships
            .sort_tier(&ctx)
            .into_iter()
            .map(|(class, ships)| {
                (
                    class,
                    ships.to_statistic(&ctx.data().expected_js, Mode::Pvp),
                )
            })
            .collect::<HashMap<ShipTier, Statistic>>()
            .into();
        let overall_data = OverallData {
            div,
            tier,
            class,
            clan,
            user: player,
        };
        let img = overall_data.render(&ctx.data().client).await?;

        let mut view = WwsView::new(partial_player);
        let mut msg = ctx
            .send(|b| {
                b.attachment(poise::serenity_prelude::AttachmentType::Bytes {
                    data: Cow::Borrowed(&img),
                    filename: "image.png".to_string(),
                })
                .components(|c| c.set_action_row(view.build()))
                .reply(true)
            })
            .await?
            .into_message()
            .await?;
        typing.stop();
        // waiting for by tier btn
        if let Some(interaction) = msg
            .await_component_interaction(ctx)
            .timeout(Duration::from_secs(60))
            .author_id(ctx.author().id)
            .await
        {
            let _tpying2 = ctx.typing().await;
            let img_2 = overall_data.render_tiers(&ctx.data().client).await?;
            // disable button first
            let _ok = interaction
                .edit_original_message(ctx, |m| {
                    m.components(|c| c.set_action_row(view.timout().build()))
                })
                .await;
            let _ok = msg
                .edit(ctx, |m| {
                    m.attachment(AttachmentType::Bytes {
                        data: Cow::Borrowed(&img_2),
                        filename: "image.png".to_string(),
                    })
                    .components(|c| c.set_action_row(view.timout().build()))
                })
                .await;
        } else {
            // timeout disable button
            let _ok = msg
                .edit(ctx, |m| {
                    m.components(|c| c.set_action_row(view.timout().build()))
                })
                .await;
        }
    } else {
        // wws ship
        let mode = args.parse_mode().unwrap_or_default();
        let ship = args.parse_ship(&ctx).await?;
        let ships = player.single_ship(&ctx, &ship).await?;
        let stats = ships.to_statistic(&ctx.data().expected_js, mode);
        dbg!(stats);
    }

    Ok(())
}

struct WwsView {
    pub player: PartialPlayer,
    by_tier_btn: CreateButton,
}

impl WwsView {
    fn new(player: PartialPlayer) -> Self {
        let btn = CreateButton::default()
            .custom_id("overall_tier")
            .style(poise::serenity_prelude::ButtonStyle::Secondary)
            .label("stats by tier")
            .to_owned();
        Self {
            player,
            by_tier_btn: btn,
        }
    }

    fn build(&self) -> CreateActionRow {
        CreateActionRow::default()
            .add_button(self.by_tier_btn.clone())
            .create_button(|b| {
                b.label("Official")
                    .url(self.player.profile_url().unwrap())
                    .style(ButtonStyle::Link)
            })
            .create_button(|b| {
                b.label("Stats & Numbers")
                    .url(self.player.wows_number_url().unwrap())
                    .style(ButtonStyle::Link)
            })
            .to_owned()
    }

    fn timout(&mut self) -> &Self {
        self.by_tier_btn.disabled(true);
        self
    }
}
