pub mod agent;
pub mod event;
pub mod item;
pub mod render;
pub mod section;
pub mod tool;

pub fn simple_id(digit: usize) -> String {
    const ALPHABET: [char; 36] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
        'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    nanoid::nanoid!(digit, &ALPHABET)
}
