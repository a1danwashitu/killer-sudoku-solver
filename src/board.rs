// sudoku-nonabit/src/board.rs から流用。
// 数字を置いたときに候補を消す範囲は、キラーでは「行・列・箱 + 同じケージ」になるため、
// 定数の MONITORS_INVERTED ではなく問題ごとの peers_inverted を引数で受け取る。
use super::constants::*;

#[derive(Clone, Debug)]
pub struct Board {
    pub numbers: [u128; HW],
    pub empty: u128,
    pub monitors: [u128; HW],
}

pub fn is_rows_valid(board: u128) -> bool {
    let mut multiple;
    let mut any;
    let mut tmp_any;

    tmp_any = (board >> 4) & 0o017017017017017017017017017;
    multiple = board & tmp_any;
    any = board | tmp_any;

    tmp_any = (any >> 2) & 0o003003003003003003003003003;
    multiple |= any & tmp_any;
    any |= tmp_any;

    tmp_any = (any >> 1) & 0o001001001001001001001001001;
    multiple |= any & tmp_any;
    any |= tmp_any;

    multiple |= any & (board >> 8) & 0o001001001001001001001001001;

    multiple == 0
}

pub fn is_cols_valid(board: u128) -> bool {
    let mut multiple;
    let mut any;
    let mut tmp_any;

    tmp_any = (board >> 36) & 0o000000000000000777777777777;
    multiple = board & tmp_any;
    any = board | tmp_any;

    tmp_any = (any >> 18) & 0o000000000000000000000777777;
    multiple |= any & tmp_any;
    any |= tmp_any;

    tmp_any = (any >> 9) & 0o000000000000000000000000777;
    multiple |= any & tmp_any;
    any |= tmp_any;

    multiple |= any & (board >> 72) & 0o000000000000000000000000777;

    multiple == 0
}

pub fn is_boxs_valid(board: u128) -> bool {
    let mut multiple;
    let mut any;
    let mut tmp_any;

    tmp_any = (board >> 9) & 0o000000777000000777000000777;
    multiple = board & tmp_any;
    any = board | tmp_any;

    tmp_any = (any >> 18) & 0o000000777000000777000000777;
    multiple |= any & tmp_any;
    any |= tmp_any;

    tmp_any = (any >> 1) & 0o000000111000000111000000111;
    multiple |= any & tmp_any;
    any |= tmp_any;

    tmp_any = (any >> 2) & 0o000000111000000111000000111;
    multiple |= any & tmp_any;

    multiple == 0
}

pub fn new_board() -> Board {
    Board {
        numbers: [0; HW],
        empty: BOARD_ALL,
        monitors: [BOARD_ALL; HW],
    }
}

// セル id に数字 num(0-indexed)を置き、影響範囲から num の候補を消す。
pub fn place(board: &mut Board, peers_inverted: &[u128; HW2], id: usize, num: usize) {
    board.numbers[num] |= 1 << id;
    board.empty ^= 1 << id;
    board.monitors[num] &= peers_inverted[id];
}

// セル id の残り候補を 9bit(bit num = 数字 num+1)で返す。
pub fn cell_candidates(board: &Board, id: usize) -> u16 {
    let mut candidates = 0;
    for num in 0..HW {
        candidates |= (((board.monitors[num] >> id) & 1) as u16) << num;
    }
    candidates
}

pub fn bitboard_to_array(bitboard: [u128; HW]) -> [usize; HW2] {
    let mut array: [usize; HW2] = [0; HW2];
    for num in 0..HW {
        for i in 0..HW2 {
            if bitboard[num] >> i & 1 == 1 {
                array[i] = num + 1;
            }
        }
    }
    array
}

// マスクの立っているビット位置を小さい順に列挙する。
pub fn bits(mut mask: u128) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if mask == 0 {
            return None;
        }
        let id = mask.trailing_zeros() as usize;
        mask &= mask - 1;
        Some(id)
    })
}
