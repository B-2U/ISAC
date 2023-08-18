use poise::serenity_prelude::UserId;

#[derive(Clone, Debug, Default)]
pub struct Patrons(pub Vec<Patron>);

impl Patrons {
    pub fn check_user(&self, discord_id: UserId) -> bool {
        self.0.iter().any(|p| p.discord_id == discord_id)
    }

    pub fn check_player(&self, uid: u64) -> bool {
        self.0.iter().any(|p| p.uid == uid)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Patron {
    pub discord_id: UserId,
    pub uid: u64,
}
