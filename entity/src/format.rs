pub mod date_time {
    use chrono::{DateTime, Local};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S"; // 在这里定义你的格式

    // 序列化逻辑
    pub fn serialize<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // 反序列化逻辑
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let t = DateTime::parse_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)?
            .with_timezone(&Local);
        Ok(t)
    }
}
