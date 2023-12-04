// some basic part of wows api reponses

use serde::{Deserialize, Serialize};

/// the status code of the response
#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    status: String,
    error: Option<String>,
}

impl Status {
    /// true if the status code is "ok"
    pub fn ok(&self) -> bool {
        match self.status.as_str() {
            "ok" => true,
            _ => false,
        }
    }
    /// return the error message, return "Unknown Error" if its None
    pub fn err_msg(self) -> String {
        self.error.as_deref().unwrap_or("Unknown Error").to_string()
    }
}
