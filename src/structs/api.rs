// some basic part of wows api reponses

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// the status code of the response
#[derive(Debug, Deserialize, Serialize)]
pub struct Status {
    status: String,
    error: Option<ErrorDetail>,
}

impl Status {
    /// true if the status code is "ok"
    pub fn ok(&self) -> bool {
        matches!(self.status.as_str(), "ok")
    }
    /// return the error message, return "Unknown Error" if its None
    pub fn err_msg(self) -> String {
        self.error
            .map(|e| e.message.clone())
            .unwrap_or("Unknown Error".to_string())
    }
}

/// Catching the error message in api
/// there's two form, string or map
///
/// `"error": "Bad Request"`
///
/// or
///
/// ` "error": {
/// "code": 504,
/// "message": "SOURCE_NOT_AVAILABLE",
/// "field": null,
/// "value": null
/// }`
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: u16,
    pub message: String,
    pub field: Option<String>,
    pub value: Option<String>,
}

/// help the ErrorDetail accepting both String and Map of field error
impl<'de> Deserialize<'de> for ErrorDetail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ErrorDetailVisitor;

        impl<'de> Visitor<'de> for ErrorDetailVisitor {
            type Value = ErrorDetail;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string or a map")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ErrorDetail {
                    code: 0,
                    message: value.to_string(),
                    field: None,
                    value: None,
                })
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                struct ErrorDetailHelper {
                    code: Option<u16>,
                    message: Option<String>,
                    field: Option<String>,
                    value: Option<String>,
                }

                let helper =
                    ErrorDetailHelper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(ErrorDetail {
                    code: helper.code.unwrap_or(0),
                    message: helper.message.unwrap_or_default(),
                    field: helper.field,
                    value: helper.value,
                })
            }
        }

        deserializer.deserialize_any(ErrorDetailVisitor)
    }
}
