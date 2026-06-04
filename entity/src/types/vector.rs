use sea_orm::{
    ColumnType, DeriveValueType, FromJsonQueryResult, FromQueryResult, QueryResult, Value,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vector(pub Vec<f32>);

// 必须为 Vector 实现从 Value 转换的逻辑（存入数据库时转为二进制）
impl From<Vector> for Value {
    fn from(v: Vector) -> Self {
        let bytes: Vec<u8> = v.0.iter().flat_map(|&f| f.to_le_bytes()).collect();
        Value::Bytes(Some(bytes))
    }
}

// 必须为 Vector 实现 TryGetable（从数据库读取时转回向量）
impl sea_orm::TryGetable for Vector {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        let vec_u8: Vec<u8> = res.try_get_by(idx)?;
        let f32_vec = vec_u8
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        Ok(Vector(f32_vec))
    }
}
// 告知 SeaORM 该类型在数据库中映射为 Blob
impl sea_orm::sea_query::Nullable for Vector {
    fn null() -> Value {
       Value::Bytes(None)
    }
}

impl sea_orm::sea_query::ValueType for Vector {
    fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        match v {
            Value::Bytes(Some(b)) => {
                let f = b
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                Ok(Vector(f))
            }
            _ => Err(sea_orm::sea_query::ValueTypeErr),
        }
    }

    fn column_type() -> ColumnType {
        ColumnType::Blob
    }
    
    fn type_name() -> String {
        "Vector".to_owned()
    }
    
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Bytes
    }
}



    // fn type_name() -> String {
    //     stringify!(OffsetDateTime).to_owned()
    // }

    // fn array_type() -> ArrayType {
    //     ArrayType::TimeDateTimeWithTimeZone
    // }

    // fn column_type() -> ColumnType {
    //     ColumnType::TimestampWithTimeZone
    // }