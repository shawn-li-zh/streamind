mod count;
mod count_pattern;

mod event_type_is;
mod includes;
mod includes_pattern;
mod index_of;
mod index_of_pattern;
mod similar;
mod time_with;

pub use {
    count::Count, count_pattern::CountPattern, event_type_is::EventTypeIs, includes::Includes,
    includes_pattern::IncludesPattern, index_of::IndexOf, index_of_pattern::IndexOfPattern,
    similar::Similar, time_with::TimeWith,
};

pub struct EventElement {
    pub event_type: String,
    pub time: String,
    pub force: bool,
    pub context: String,
    pub embedding: Vec<f32>,
}
