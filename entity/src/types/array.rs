use sea_orm::{DeriveValueType, FromJsonQueryResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct StringArray(pub Vec<String>);
