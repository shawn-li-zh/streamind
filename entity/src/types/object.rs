use std::collections::HashMap;

use sea_orm::{DeriveValueType, FromJsonQueryResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct JsonObject(pub serde_json::value::Map<String, serde_json::Value>);
