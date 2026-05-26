mod count;
mod event_type_is;
mod includes;
mod index_of;
mod matches;
mod similar;
mod time_with;

pub use {
    count::Count, event_type_is::EventTypeIs, includes::Includes, index_of::IndexOf,
    matches::Matches, similar::Similar, time_with::TimeWith,
};

pub struct EventElement {
    pub event_type: String,
    pub time: String,
    pub force: bool,
    pub context: String,
    pub embedding: Vec<f32>,
}
