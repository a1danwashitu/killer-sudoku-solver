// sudoku-nonabit/src/naked_single.rs から流用(影響範囲を peers_inverted で受け取る点のみ変更)。
use super::board::*;
use super::constants::*;

// 戻り値は (矛盾を検出したか, 1 マスも埋まらなかったか)。
pub fn naked_single(board: &mut Board, peers_inverted: &[u128; HW2]) -> (bool, bool) {
    let mut single0;
    let single1;
    let mut single2;
    let single3;
    let mut any0;
    let any1;
    let mut any2;
    let any3;

    single0 = board.monitors[0] ^ board.monitors[1];
    single1 = board.monitors[2] ^ board.monitors[3];
    single2 = board.monitors[4] ^ board.monitors[5];
    single3 = board.monitors[6] ^ board.monitors[7];
    any0 = board.monitors[0] | board.monitors[1];
    any1 = board.monitors[2] | board.monitors[3];
    any2 = board.monitors[4] | board.monitors[5];
    any3 = board.monitors[6] | board.monitors[7];

    single0 = (single0 ^ single1) & (any0 ^ any1);
    single2 = (single2 ^ single3) & (any2 ^ any3);
    any0 = any0 | any1;
    any2 = any2 | any3;

    single0 = (single0 ^ single2) & (any0 ^ any2);
    any0 = any0 | any2;

    single0 = (single0 ^ board.monitors[8]) & (any0 ^ board.monitors[8]);

    let filled = single0 & board.empty;
    if filled == 0 {
        return (false, true);
    }
    for i in 0..HW {
        let mut filled_num = filled & board.monitors[i];
        if filled_num == 0 {
            continue;
        }
        if !(is_rows_valid(filled_num) && is_cols_valid(filled_num) && is_boxs_valid(filled_num)) {
            return (true, false);
        }
        board.numbers[i] |= filled_num;
        board.empty ^= filled_num;
        loop {
            let id = filled_num.trailing_zeros() as usize;
            if id == 128 {
                break;
            }
            board.monitors[i] &= peers_inverted[id];
            filled_num ^= 1 << id;
        }
    }

    (false, false)
}
