use crate::{dc_utils::Args, Context, Error};

// #[poise::command(slash_command, rename = "test")]
// pub async fn test_slash(
//     ctx: Context<'_>,
//     #[autocomplete = "autocomplete_number11"] args: Option<u64>,
// ) -> Result<(), Error> {
//     Ok(())
// }

#[poise::command(prefix_command)]
pub async fn test(ctx: Context<'_>, #[rest] args: Option<Args>) -> Result<(), Error> {
    Ok(())
}

pub async fn autocomplete_number11(
    ctx: Context<'_>,
    input: &str,
) -> impl Iterator<Item = poise::AutocompleteChoice<u64>> {
    vec![0, 1, 2, 3, 4]
        .into_iter()
        .map(|i| poise::AutocompleteChoice {
            name: i.to_string(),
            value: i,
        })
}

// pub fn test_slash() -> ::poise::Command<
//     <Context<'static> as poise::_GetGenerics>::U,
//     <Context<'static> as poise::_GetGenerics>::E,
// > {
//     pub async fn inner(ctx: Context<'_>, args: Option<u64>) -> Result<(), Error> {
//         Ok(())
//     }
//     ::poise::Command {
//         prefix_action: None,
//         slash_action: Some(|ctx| {
//             Box::pin(async move {
//                 #[allow(clippy::needless_question_mark)]
//                 let (args,) = async {
//                     use ::poise::SlashArgumentHack;
//                     let (ctx, interaction, args) =
//                         (ctx.serenity_context, ctx.interaction, ctx.args);
//                     Ok::<_, ::poise::SlashArgError>((
//                         if let Some(arg) = args.iter().find(|arg| arg.name == "args") {
//                             let arg = arg.value.as_ref().ok_or(
//                                 ::poise::SlashArgError::CommandStructureMismatch(
//                                     "expected argument value",
//                                 ),
//                             )?;
//                             Some(
//                                 {
//                                     use ::poise::SlashArgumentHack as _;
//                                     (&&std::marker::PhantomData::<u64>).extract(
//                                         ctx,
//                                         interaction,
//                                         arg,
//                                     )
//                                 }
//                                 .await?,
//                             )
//                         } else {
//                             None
//                         },
//                     ))
//                 }
//                 .await
//                 .map_err(|error| match error {
//                     poise::SlashArgError::CommandStructureMismatch(description) => {
//                         poise::FrameworkError::CommandStructureMismatch { ctx, description }
//                     }
//                     poise::SlashArgError::Parse { error, input } => {
//                         poise::FrameworkError::ArgumentParse {
//                             ctx: ctx.into(),
//                             error,
//                             input: Some(input),
//                         }
//                     }
//                 })?;
//                 if !ctx.framework.options.manual_cooldowns {
//                     ctx.command
//                         .cooldowns
//                         .lock()
//                         .unwrap()
//                         .start_cooldown(ctx.into());
//                 }
//                 inner(ctx.into(), args)
//                     .await
//                     .map_err(|error| poise::FrameworkError::Command {
//                         error,
//                         ctx: ctx.into(),
//                     })
//             })
//         }),
//         context_menu_action: None,
//         subcommands: ::alloc::vec::Vec::new(),
//         name: "test".to_string(),
//         name_localizations: std::collections::HashMap::from([]),
//         qualified_name: String::from("test"),
//         identifying_name: String::from("test_slash"),
//         category: None,
//         description: None,
//         description_localizations: std::collections::HashMap::from([]),
//         help_text: None,
//         hide_in_help: false,
//         cooldowns: std::sync::Mutex::new(::poise::Cooldowns::new(::poise::CooldownConfig {
//             global: None.map(std::time::Duration::from_secs),
//             user: None.map(std::time::Duration::from_secs),
//             guild: None.map(std::time::Duration::from_secs),
//             channel: None.map(std::time::Duration::from_secs),
//             member: None.map(std::time::Duration::from_secs),
//         })),
//         reuse_response: false,
//         default_member_permissions: poise::serenity_prelude::Permissions::empty(),
//         required_permissions: poise::serenity_prelude::Permissions::empty(),
//         required_bot_permissions: poise::serenity_prelude::Permissions::empty(),
//         owners_only: false,
//         guild_only: false,
//         dm_only: false,
//         nsfw_only: false,
//         checks: ::alloc::vec::Vec::new(),
//         on_error: None,
//         parameters: <[_]>::into_vec(
//             #[rustc_box]
//             ::alloc::boxed::Box::new([::poise::CommandParameter {
//                 name: "args".to_string(),
//                 name_localizations: ::alloc::vec::Vec::new().into_iter().collect(),
//                 description: None,
//                 description_localizations: ::alloc::vec::Vec::new().into_iter().collect(),
//                 required: false,
//                 channel_types: None,
//                 type_setter: Some(|o| {
//                     {
//                         use ::poise::SlashArgumentHack as _;
//                         (&&std::marker::PhantomData::<u64>).create(o)
//                     };
//                 }),
//                 choices: {
//                     use ::poise::SlashArgumentHack as _;
//                     (&&std::marker::PhantomData::<u64>).choices()
//                 },
//                 autocomplete_callback: Some(
//                     |ctx: poise::ApplicationContext<'_, _, _>, partial: &str| {
//                         Box::pin(async move {
//                             use ::poise::futures_util::{Stream, StreamExt};
//                             let choices_stream =
//                                 match autocomplete_number11(ctx.into(), partial).await {
//                                     value => {
//                                         use ::poise::IntoStream as _;
//                                         (&&::poise::IntoStreamWrap(&value)).converter()(value)
//                                     }
//                                 };
//                             let choices_json = choices_stream
//                                 .take(25)
//                                 .map(|value| poise::AutocompleteChoice::from(value))
//                                 .map(|choice| {
//                                     ::serde_json::Value::Object({
//                                         let mut object = ::serde_json::Map::new();
//                                         let _ = object.insert(
//                                             ("name").into(),
//                                             ::serde_json::to_value(&choice.name).unwrap(),
//                                         );
//                                         let _ = object.insert(
//                                             ("value").into(),
//                                             ::serde_json::to_value(&choice.value).unwrap(),
//                                         );
//                                         object
//                                     })
//                                 })
//                                 .collect()
//                                 .await;
//                             let mut response =
//                                 poise::serenity_prelude::CreateAutocompleteResponse::default();
//                             response.set_choices(poise::serenity_prelude::json::Value::Array(
//                                 choices_json,
//                             ));
//                             Ok(response)
//                         })
//                     },
//                 ),
//             }]),
//         ),
//         custom_data: Box::new(()),
//         aliases: &[],
//         invoke_on_edit: false,
//         track_deletion: false,
//         broadcast_typing: false,
//         context_menu_name: None,
//         ephemeral: false,
//         __non_exhaustive: (),
//     }
// }
