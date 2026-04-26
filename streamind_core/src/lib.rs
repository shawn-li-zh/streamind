pub mod agent;
pub mod event;
pub mod item;
pub mod section;
pub mod tool;

pub fn simple_id(digit: usize) -> String {
    const ALPHABET: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    nanoid::nanoid!(digit, &ALPHABET)
}
