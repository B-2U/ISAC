use std::{collections::HashMap, env, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use poise::serenity_prelude::{Http, RoleId};
use tracing::warn;

use crate::{
    utils::{
        structs::{Linked, Patron, Patrons},
        LoadSaveFromJson,
    },
    Error,
};

pub async fn patron_updater(http: Arc<Http>, patrons_arc: Arc<RwLock<Patrons>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    static GUILD_ID: Lazy<u64> = Lazy::new(|| env::var("GUILD_ID").unwrap().parse().unwrap());
    static PATREON_ID: Lazy<RoleId> =
        Lazy::new(|| RoleId(env::var("PATREON_ROLE_ID").unwrap().parse().unwrap()));
    static SUP_ID: Lazy<RoleId> =
        Lazy::new(|| RoleId(env::var("SUPPORTER_ROLE_ID").unwrap().parse().unwrap()));

    async fn get_patrons(http: &Arc<Http>) -> Result<Patrons, Error> {
        let linked_js: HashMap<_, _> = Linked::load_json().await.into();
        let guild = http.get_guild(*GUILD_ID).await?;
        let patron_vec = guild
            .members(http, None, None)
            .await?
            .into_iter()
            .filter(|m| m.roles.contains(&PATREON_ID) || m.roles.contains(&SUP_ID))
            .map(|m| Patron {
                uid: linked_js
                    .get(&m.user.id)
                    .map(|linked_user| linked_user.uid)
                    .unwrap_or(0),
                discord_id: m.user.id,
            })
            .collect::<Vec<_>>();
        Ok(Patrons(patron_vec))
    }
    loop {
        interval.tick().await;
        match get_patrons(&http).await {
            Ok(patrons) => *patrons_arc.write() = patrons,
            Err(err) => warn!("patrons task fail!, {err}"),
        }
    }
}
