pub const HW: usize = 9;
pub const HW2: usize = HW * HW;
pub const HWRT: usize = 3;

pub const ROW: u128 = 0o777;
pub const COL: u128 = 0o001001001001001001001001001;
pub const BOX: u128 = 0o007007007;

pub const BOARD_ALL: u128 = 0o777777777777777777777777777;

pub const ROWS: [u128; HW] = {
    let mut i = 0;
    let mut rows: [u128; HW] = [0; HW];
    while i < HW {
        rows[i] = ROW << i * HW;
        i += 1;
    }
    rows
};

pub const COLS: [u128; HW] = {
    let mut i = 0;
    let mut cols: [u128; HW] = [0; HW];
    while i < HW {
        cols[i] = COL << i;
        i += 1;
    }
    cols
};

pub const BOXS: [u128; HW] = {
    let mut i = 0;
    let mut boxs: [u128; HW] = [0; HW];
    while i < HW {
        boxs[i] = BOX << i / HWRT * HWRT * HW + i % HWRT * HWRT;
        i += 1;
    }
    boxs
};

pub const MONITORS: [u128; HW2] = {
    let mut monitors: [u128; HW2] = [0; HW2];
    let mut i = 0;
    while i < HW2 {
        monitors[i] |= ROWS[i / HW];
        monitors[i] |= COLS[i % HW];
        monitors[i] |= BOXS[i / (HWRT * HW) * HWRT + i % HW / HWRT];
        i = i + 1;
    }
    monitors
};

pub const MONITORS_INVERTED: [u128; HW2] = {
    let mut monitors_inverted: [u128; HW2] = [0; HW2];
    let mut i = 0;
    while i < HW2 {
        monitors_inverted[i] = !MONITORS[i] & BOARD_ALL;
        i = i + 1;
    }
    monitors_inverted
};

// 行・列・箱の 27 単位。ケージ pointing と 45 の法則で使う。
pub const UNITS: [u128; 3 * HW] = {
    let mut units: [u128; 3 * HW] = [0; 3 * HW];
    let mut i = 0;
    while i < HW {
        units[i] = ROWS[i];
        units[HW + i] = COLS[i];
        units[2 * HW + i] = BOXS[i];
        i += 1;
    }
    units
};
