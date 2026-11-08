pub mod board;
pub mod brute;
pub mod combos;
pub mod constants;
pub mod display;
pub mod generate;
pub mod hidden_single;
pub mod logic;
pub mod naked_single;
pub mod propagate;
pub mod puzzle;
pub mod search;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(test)]
mod tests;
