// sudoku-nonabit/src/hidden_single.rs から流用(影響範囲を peers_inverted で受け取る点のみ変更)。
use super::board::*;
use super::constants::*;

const HIT_ROWS: [u128; 1 << HW] = {
    let mut hit_rows: [u128; 1 << HW] = [0; 1 << HW];
    let mut i = 0;
    while i < 1 << HW {
        let mut j = 0;
        while j < HW {
            if i >> j & 1 == 1 {
                hit_rows[i] |= ROWS[j]
            }
            j += 1;
        }
        i += 1;
    }
    hit_rows
};

const HIT_COLS: [u128; 1 << HW] = {
    let mut hit_cols: [u128; 1 << HW] = [0; 1 << HW];
    let mut i = 0;
    while i < 1 << HW {
        let mut j = 0;
        while j < HW {
            if i >> j & 1 == 1 {
                hit_cols[i] |= COLS[j]
            }
            j += 1;
        }
        i += 1;
    }
    hit_cols
};

const HIT_BOXS: [u128; 1 << HW] = {
    let mut hit_boxs: [u128; 1 << HW] = [0; 1 << HW];
    let mut i = 0;
    while i < 1 << HW {
        let mut j = 0;
        while j < HW {
            if i >> j & 1 == 1 {
                hit_boxs[i] |= BOXS[j]
            }
            j += 1;
        }
        i += 1;
    }
    hit_boxs
};

pub fn hidden_single(board: &mut Board, peers_inverted: &[u128; HW2]) -> (bool, bool) {
    let mut filled = 0;

    for i in 0..HW {

        let filled_rows = hidden_single_rows(board.monitors[i] & board.empty);

        board.numbers[i] |= filled_rows;
        board.empty ^= filled_rows;

        let mut tmp = filled_rows;
        loop {
            let id = tmp.trailing_zeros() as usize;
            if id == 128 {
                break;
            }
            board.monitors[i] &= peers_inverted[id];
            tmp ^= 1 << id;
        }

        let mut filled_num = filled_rows;

        let filled_cols = hidden_single_cols(board.monitors[i] & board.empty);

        board.numbers[i] |= filled_cols;
        board.empty ^= filled_cols;

        tmp = filled_cols;
        loop {
            let id = tmp.trailing_zeros() as usize;
            if id == 128 {
                break;
            }
            board.monitors[i] &= peers_inverted[id];
            tmp ^= 1 << id;
        }

        filled_num |= filled_cols;

        let filled_boxs = hidden_single_boxs(board.monitors[i] & board.empty);

        board.numbers[i] |= filled_boxs;
        board.empty ^= filled_boxs;

        let mut tmp = filled_boxs;
        loop {
            let id = tmp.trailing_zeros() as usize;
            if id == 128 {
                break;
            }
            board.monitors[i] &= peers_inverted[id];
            tmp ^= 1 << id;
        }

        filled_num |= filled_boxs;

        if !(is_rows_valid(filled_num) && is_cols_valid(filled_num) && is_boxs_valid(filled_num)) {
            return (true, false);
        }

        filled |= filled_num;
    }
    (false, filled == 0)
}

fn hidden_single_rows(board: u128) -> u128 {
    let mut single;
    let mut any;
    let mut tmp_single;
    let mut tmp_any;

    tmp_any = (board >> 4) & 0o017017017017017017017017017;
    single = board ^ tmp_any;
    any = board | tmp_any;

    tmp_single = (single >> 2) & 0o003003003003003003003003003;
    tmp_any = (any >> 2) & 0o003003003003003003003003003;
    single = (single ^ tmp_single) & (any ^ tmp_any);
    any = any | tmp_any;

    tmp_single = (single >> 1) & 0o001001001001001001001001001;
    tmp_any = (any >> 1) & 0o001001001001001001001001001;
    single = (single ^ tmp_single) & (any ^ tmp_any);
    any = any | tmp_any;

    tmp_any = board >> 8;
    single = (single ^ tmp_any) & (any ^ tmp_any) & 0o001001001001001001001001001;

    board & HIT_ROWS[((single * 0x808080808080808080000000000000) >> 119) as usize]
}

fn hidden_single_cols(board: u128) -> u128 {
    let mut single;
    let mut any;
    let mut tmp_single;
    let mut tmp_any;

    tmp_any = (board >> 36) & 0o000000000000000777777777777;
    single = board ^ tmp_any;
    any = board | tmp_any;

    tmp_single = (single >> 18) & 0o000000000000000000000777777;
    tmp_any = (any >> 18) & 0o000000000000000000000777777;
    single = (single ^ tmp_single) & (any ^ tmp_any);
    any = any | tmp_any;

    tmp_single = (single >> 9) & 0o000000000000000000000000777;
    tmp_any = (any >> 9) & 0o000000000000000000000000777;
    single = (single ^ tmp_single) & (any ^ tmp_any);
    any = any | tmp_any;

    tmp_any = board >> 72;
    single = (single ^ tmp_any) & (any ^ tmp_any);

    board & HIT_COLS[(single & 0o000000000000000000000000777) as usize]
}

fn hidden_single_boxs(board: u128) -> u128 {
    let mut single;
    let mut any;
    let tmp_single;
    let mut tmp_any;

    tmp_any = (board >> 1) & 0o111111111111111111111111111;
    single = board ^ tmp_any;
    any = board | tmp_any;

    tmp_any = (board >> 2) & 0o111111111111111111111111111;
    single = (single ^ tmp_any) & (any ^ tmp_any);
    any = any | tmp_any;

    tmp_single = (single >> 9) & 0o000000111000000111000000111;
    tmp_any = (any >> 9) & 0o000000111000000111000000111;
    single = (single ^ tmp_single) & (any ^ tmp_any);
    any = any | tmp_any;

    single = (single ^ (single >> 18)) & (any ^ (any >> 18)) & 0o000000111000000111000000111;

    single = (single * 0x54) & 0o000000700000000700000000700;
    board & HIT_BOXS[((single * 0x20000020000020000000000000000) >> 119) as usize]
}
