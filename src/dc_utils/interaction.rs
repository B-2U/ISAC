use poise::{
    async_trait,
    serenity_prelude::{
        CreateInteractionResponseData, Error, Http, InteractionResponseType,
        MessageComponentInteraction,
    },
};

#[async_trait]
pub trait InteractionAddon {
    async fn edit_original_message<'a, F>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        f: F,
    ) -> Result<(), Error>
    where
        F: Send,
        for<'b> F: FnOnce(
            &'b mut CreateInteractionResponseData<'a>,
        ) -> &'b mut CreateInteractionResponseData<'a>;
}

/// a trait for `reply`
#[async_trait]
impl InteractionAddon for MessageComponentInteraction {
    async fn edit_original_message<'a, F>(
        &self,
        http: impl AsRef<Http> + Send + Sync,
        f: F,
    ) -> Result<(), Error>
    where
        F: Send,
        for<'b> F: FnOnce(
            &'b mut CreateInteractionResponseData<'a>,
        ) -> &'b mut CreateInteractionResponseData<'a>,
    {
        self.create_interaction_response(http, |m| {
            m.kind(InteractionResponseType::UpdateMessage);
            m.interaction_response_data(|builder| f(builder))
        })
        .await
    }
}
