// テスト用の素朴な総当たりソルバー。ビットボード・組み合わせ表・45 の法則を一切使わず、
// 高速ソルバーの解の個数を照合する基準にする。
use super::constants::*;
use super::puzzle::Puzzle;

struct State {
    grid: [usize; HW2],
    cage_of: [usize; HW2],
    cage_sum: Vec<u32>,
    cage_size: Vec<u32>,
    cage_filled_sum: Vec<u32>,
    cage_filled_count: Vec<u32>,
    cage_used: Vec<u16>,
    row_used: [u16; HW],
    col_used: [u16; HW],
    box_used: [u16; HW],
}

fn box_of(id: usize) -> usize {
    id / 27 * 3 + id % 9 / 3
}

impl State {
    fn can_place(&self, id: usize, n: usize) -> bool {
        let bit = 1 << n;
        let c = self.cage_of[id];
        if (self.row_used[id / 9] | self.col_used[id % 9] | self.box_used[box_of(id)] | self.cage_used[c]) & bit != 0 {
            return false;
        }
        // 置いた後の残り合計が、残りマス数で作れる範囲に入るか(使用済み数字は考慮しない粗い判定)。
        let total = self.cage_filled_sum[c] + n as u32;
        if total > self.cage_sum[c] {
            return false;
        }
        let rem = self.cage_sum[c] - total;
        let r = self.cage_size[c] - self.cage_filled_count[c] - 1;
        rem >= r * (r + 1) / 2 && rem <= r * (19 - r) / 2
    }

    fn set(&mut self, id: usize, n: usize, on: bool) {
        let bit = 1 << n;
        let c = self.cage_of[id];
        self.row_used[id / 9] ^= bit;
        self.col_used[id % 9] ^= bit;
        self.box_used[box_of(id)] ^= bit;
        self.cage_used[c] ^= bit;
        if on {
            self.grid[id] = n;
            self.cage_filled_sum[c] += n as u32;
            self.cage_filled_count[c] += 1;
        } else {
            self.grid[id] = 0;
            self.cage_filled_sum[c] -= n as u32;
            self.cage_filled_count[c] -= 1;
        }
    }

    fn dfs(&mut self, max_count: usize, count: &mut usize, first: &mut Option<[usize; HW2]>) {
        // 候補数が最小の空きマスを選ぶ。
        let mut best: Option<(usize, usize)> = None;
        for id in 0..HW2 {
            if self.grid[id] != 0 {
                continue;
            }
            let n = (1..=HW).filter(|&n| self.can_place(id, n)).count();
            if best.map_or(true, |(_, m)| n < m) {
                best = Some((id, n));
            }
        }
        let Some((id, _)) = best else {
            *count += 1;
            if first.is_none() {
                *first = Some(self.grid);
            }
            return;
        };
        for n in 1..=HW {
            if *count >= max_count {
                return;
            }
            if self.can_place(id, n) {
                self.set(id, n, true);
                self.dfs(max_count, count, first);
                self.set(id, n, false);
            }
        }
    }
}

pub fn count_solutions(puzzle: &Puzzle, max_count: usize) -> (usize, Option<[usize; HW2]>) {
    let mut state = State {
        grid: [0; HW2],
        cage_of: puzzle.cage_ids(),
        cage_sum: puzzle.cages.iter().map(|c| c.sum).collect(),
        cage_size: puzzle.cages.iter().map(|c| c.cells.count_ones()).collect(),
        cage_filled_sum: vec![0; puzzle.cages.len()],
        cage_filled_count: vec![0; puzzle.cages.len()],
        cage_used: vec![0; puzzle.cages.len()],
        row_used: [0; HW],
        col_used: [0; HW],
        box_used: [0; HW],
    };
    for id in 0..HW2 {
        let n = puzzle.givens[id];
        if n != 0 {
            if !state.can_place(id, n) {
                return (0, None);
            }
            state.set(id, n, true);
        }
    }
    let mut count = 0;
    let mut first = None;
    state.dfs(max_count, &mut count, &mut first);
    (count, first)
}
