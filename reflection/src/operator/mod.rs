
pub mod count;
pub mod includes;
pub mod index_of;
pub mod matches;
pub mod similar;
pub mod event_type_is;
pub mod time_with;

pub struct EventElement {
    pub event_type: String,
    pub time: String,
    // pub time: chrono::DateTime<chrono::Local>,
    pub force: bool,
    pub context: String,
    pub embedding: Vec<f32>,
}
