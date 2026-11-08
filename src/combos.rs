// ケージに入りうる数字の組み合わせ表。
// 1〜9 の部分集合(2^9 = 512 通り、bit num = 数字 num+1)を (要素数, 合計) ごとに
// 並べ替えてコンパイル時に作っておき、combos(size, sum) でスライスとして引く。
use super::constants::*;

const MAX_SUM: usize = 45;
const KEYS: usize = (HW + 1) * (MAX_SUM + 1);

const fn mask_sum(mask: usize) -> usize {
    let mut sum = 0;
    let mut num = 0;
    while num < HW {
        if mask >> num & 1 == 1 {
            sum += num + 1;
        }
        num += 1;
    }
    sum
}

const fn key(mask: usize) -> usize {
    mask.count_ones() as usize * (MAX_SUM + 1) + mask_sum(mask)
}

// COMBO_START[k]..COMBO_START[k + 1] がキー k の組み合わせの範囲。
const COMBO_START: [usize; KEYS + 1] = {
    let mut start = [0; KEYS + 1];
    let mut mask = 0;
    while mask < 1 << HW {
        start[key(mask) + 1] += 1;
        mask += 1;
    }
    let mut k = 0;
    while k < KEYS {
        start[k + 1] += start[k];
        k += 1;
    }
    start
};

const COMBO_LIST: [u16; 1 << HW] = {
    let mut list = [0; 1 << HW];
    let mut next = COMBO_START;
    let mut mask = 0;
    while mask < 1 << HW {
        let k = key(mask);
        list[next[k]] = mask as u16;
        next[k] += 1;
        mask += 1;
    }
    list
};

// size 個の相異なる数字で合計 sum になる組み合わせの一覧。
pub fn combos(size: usize, sum: u32) -> &'static [u16] {
    let sum = sum as usize;
    if size > HW || sum > MAX_SUM {
        return &[];
    }
    let k = size * (MAX_SUM + 1) + sum;
    &COMBO_LIST[COMBO_START[k]..COMBO_START[k + 1]]
}

// size 個の相異なる数字で作れる合計の最小値と最大値。
pub fn sum_range(size: usize) -> (u32, u32) {
    let size = size as u32;
    (size * (size + 1) / 2, size * (19 - size) / 2)
}
