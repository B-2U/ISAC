use std::{
    env::{self, VarError},
    sync::Arc,
    time::Duration,
};

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use poise::serenity_prelude::{Http, RoleId};
use tracing::warn;

use crate::{
    structs::{Linked, Patron, Patrons},
    utils::LoadSaveFromJson,
    Error,
};

pub async fn patron_updater(http: Arc<Http>, patrons_arc: Arc<RwLock<Patrons>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(300));
    static IDS: Lazy<Option<Ids>> = Lazy::new(|| load_ids().ok());

    if IDS.is_some() {
        loop {
            interval.tick().await;
            match get_patrons(&http, IDS.as_ref().unwrap()).await {
                Ok(patrons) => *patrons_arc.write() = patrons,
                Err(err) => warn!("patrons task fail!, err: \n{err}"),
            }
        }
    } else {
        warn!("GUILD_ID not in env, disable patreon updater");
    };
}

async fn get_patrons(http: &Arc<Http>, ids: &Ids) -> Result<Patrons, Error> {
    let linked_js = Linked::load_json().await;
    let guild = http.get_guild(ids.guild_id).await?;
    let patron_vec = guild
        .members(http, None, None)
        .await?
        .into_iter()
        .filter(|m| m.roles.contains(&ids.patreon_id) || m.roles.contains(&ids.sup_id))
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

fn load_ids() -> Result<Ids, VarError> {
    let guild_id: u64 = env::var("GUILD_ID")?.parse().unwrap();
    // set role ids to 0 if wasn't specific
    let patreon_id: RoleId = RoleId(
        env::var("PATREON_ROLE_ID")
            .map(|v| v.parse().unwrap())
            .unwrap_or_default(),
    );
    let sup_id: RoleId = RoleId(
        env::var("SUPPORTER_ROLE_ID")
            .map(|v| v.parse().unwrap())
            .unwrap_or_default(),
    );
    Ok(Ids {
        guild_id,
        patreon_id,
        sup_id,
    })
}

struct Ids {
    guild_id: u64,
    patreon_id: RoleId,
    sup_id: RoleId,
}
