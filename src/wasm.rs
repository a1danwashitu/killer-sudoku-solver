//! ブラウザで問題を生成するための WebAssembly の出口。
//! wasm-bindgen は使わず、数値と線形メモリだけでやり取りする:
//! JS は `generate(seed_hi, seed_lo, max_size)` で 1 問生成し、返り値のバイト数だけ
//! `output_ptr()` から読むと `Puzzle::to_line` の文字列が得られる。

use std::cell::RefCell;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::generate::{generate as generate_puzzle, GenOptions};

thread_local! {
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[no_mangle]
pub extern "C" fn generate(seed_hi: u32, seed_lo: u32, max_size: u32) -> u32 {
    let mut rng = StdRng::seed_from_u64(((seed_hi as u64) << 32) | seed_lo as u64);
    let options = GenOptions { max_cage_size: (max_size as usize).clamp(1, 9) };
    let line = generate_puzzle(&mut rng, &options).to_line();
    OUTPUT.with(|out| {
        let mut out = out.borrow_mut();
        out.clear();
        out.extend_from_slice(line.as_bytes());
        out.len() as u32
    })
}

#[no_mangle]
pub extern "C" fn output_ptr() -> *const u8 {
    OUTPUT.with(|out| out.borrow().as_ptr())
}

// 乱数の種は JS から渡すので、OS の乱数源は使わない(呼ばれたらエラーにする)。
fn no_entropy(_: &mut [u8]) -> Result<(), getrandom::Error> {
    Err(getrandom::Error::UNSUPPORTED)
}
getrandom::register_custom_getrandom!(no_entropy);
