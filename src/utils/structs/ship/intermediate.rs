use std::collections::HashMap;

use serde::Deserialize;

use crate::utils::structs::{ShipId, ShipModeStatsPair};

#[derive(Debug, Deserialize)]
struct PlayerStats {
    statistics: HashMap<ShipId, ShipModeStatsPair>,
    name: String, // player IGN. Don't care.
    hidden_profile: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct VortexShipResponse {
    status: String,                     // known values: `ok`, `error`.
    error: Option<String>,              // error message.
    data: HashMap<String, PlayerStats>, // key is player UID. Don't care.
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::VortexShipResponse;

    #[test]
    fn vortex_ship_response_can_deserialize() {
        let json = fs::read_to_string("test_res/vortex-ship-response.json").unwrap();
        let _res: VortexShipResponse = serde_json::from_str(&json).unwrap();
    }
}
