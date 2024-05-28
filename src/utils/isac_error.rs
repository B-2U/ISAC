use std::fmt::{self, Display};

use crate::{
    structs::{Mode, PartialClan, Region},
    Error,
};

#[derive(Debug, thiserror::Error)]
pub enum IsacError {
    #[error("IsacError: {0}")]
    Help(#[from] IsacHelp),
    #[error("IsacError: {0}")]
    Info(#[from] IsacInfo),
    #[error("IsacError: Cancelled")]
    Cancelled,
    #[error("IsacError: {0}")]
    UnknownError(#[from] Error),
}

#[derive(Debug, strum::Display, thiserror::Error)]
pub enum IsacHelp {
    LackOfArguments,
}
#[derive(Debug, thiserror::Error)]
pub enum IsacInfo {
    UserNotLinked {
        user_name: Option<String>, // give None if its author himself
    },
    UserNoClan {
        user_name: Option<String>, // give None if its author himself
    },
    TooShortIgn {
        ign: String,
    },
    InvalidIgn {
        ign: String,
    },
    PlayerIgnNotFound {
        ign: String,
        region: Region,
    },
    PlayerHidden {
        ign: String,
    },
    PlayerNoBattle {
        ign: String,
    },
    PlayerNoBattleShip {
        ign: String,
        ship_name: String,
        mode: Mode,
    },

    InvalidClan {
        clan: String,
    },
    ClanNotFound {
        clan: String,
        region: Region,
    },
    ClanNoBattle {
        clan: PartialClan,
        season: u32,
    },

    ShipNotFound {
        ship_name: String,
    },
    APIError {
        msg: String,
    },
    GeneralError {
        msg: String,
    },
    NeedPremium {
        msg: String,
    },
    AutoCompleteError,

    EmbedPermission,
}

impl Display for IsacInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            IsacInfo::UserNotLinked { user_name } => match user_name.as_ref() {
                Some(user_name) => {
                    format!("**{user_name}** haven't linked to any wows account yet")
                }
                None => "You haven't linked your account yet.\nEnter `/link`".to_string(),
            },
            IsacInfo::UserNoClan { user_name } => match user_name.as_ref() {
                Some(user_name) => {
                    format!("**{user_name}** is not in a clan")
                }
                None => "You are not in a clan".to_string(),
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
                clan.tag.replace('_', r"\_"),
                clan.region,
                season
            ),
            IsacInfo::NeedPremium { msg } => format!("{msg}\n{PREMIUM}"),
            IsacInfo::EmbedPermission => "❌ This error means ISAC don't have to permission to send embed here, please check the **Embed Links** in the permission setting, \nOr you can just re-invite ISAC in discord to let it grant the permission".to_string(),
        };
        write!(f, "{}", msg)
    }
}

impl From<reqwest::Error> for IsacError {
    fn from(err: reqwest::Error) -> Self {
        IsacInfo::APIError {
            msg: err.to_string(),
        }
        .into()
    }
}

const PREMIUM: &str =
    "Seems you haven't join our Patreon, or link your discord account on Patreon yet :(
If you do like ISAC, [take a look?]( https://www.patreon.com/ISAC_bot )";
