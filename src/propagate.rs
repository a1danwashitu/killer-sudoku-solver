// 制約伝播。分岐を伴わない推論だけを不動点まで回す。
// ここでの推論はすべて健全(解を消さない)なので、伝播だけで全マス埋まれば解は一意。
use super::board::*;
use super::combos::combos;
use super::constants::*;
use super::hidden_single::hidden_single;
use super::naked_single::naked_single;
use super::puzzle::Puzzle;

// 45 の法則で作る仮想制約の最大マス数。伝播の作業配列が 9 要素なのでこれ以下。
const RULE45_MAX_CELLS: u32 = HW as u32;

#[derive(Clone, Debug)]
pub struct SumConstraint {
    pub cells: u128,
    pub sum: u32,
    // true なら cells 内で数字が重複しない(ケージ、または全マスが互いに同じ単位にある仮想制約)。
    pub distinct: bool,
}

pub struct Contradiction;

pub struct Context {
    // セル id に数字を置いても候補が変わらないマス(行・列・箱・ケージの外)。
    pub peers_inverted: [u128; HW2],
    pub constraints: Vec<SumConstraint>,
    // 45 の法則の計算で合計が負になるなど、伝播以前に解がないと分かった。
    pub infeasible: bool,
}

impl Context {
    // ケージなしの通常の数独(完成盤面の生成に使う)。
    pub fn sudoku() -> Self {
        Self {
            peers_inverted: MONITORS_INVERTED,
            constraints: vec![],
            infeasible: false,
        }
    }

    pub fn new(puzzle: &Puzzle, rule45: bool) -> Self {
        let mut context = Self::sudoku();
        for cage in &puzzle.cages {
            for id in bits(cage.cells) {
                context.peers_inverted[id] &= !cage.cells;
            }
            context.constraints.push(SumConstraint {
                cells: cage.cells,
                sum: cage.sum,
                distinct: true,
            });
        }
        if rule45 {
            context.add_rule45(puzzle);
        }
        context
    }

    fn add_constraint(&mut self, cells: u128, sum: i64) {
        if sum < 0 {
            self.infeasible = true;
            return;
        }
        if cells == 0 || cells.count_ones() > RULE45_MAX_CELLS {
            return;
        }
        if self.constraints.iter().any(|c| c.cells == cells) {
            return;
        }
        let distinct = bits(cells).all(|id| cells & self.peers_inverted[id] == 0);
        self.constraints.push(SumConstraint {
            cells,
            sum: sum as u32,
            distinct,
        });
    }

    // 連続する行の束・列の束・各箱について、innies(領域内でケージに収まりきらないマス)と
    // outies(領域からはみ出したマス)の合計を仮想制約として追加する。
    fn add_rule45(&mut self, puzzle: &Puzzle) {
        let mut regions: Vec<(u128, i64)> = vec![];
        for lines in [&ROWS, &COLS] {
            for start in 0..HW {
                let mut region = 0;
                for end in start..HW {
                    region |= lines[end];
                    // 9 本すべての束は盤面全体なので innies/outies が出ない。
                    if end - start + 1 < HW {
                        regions.push((region, (end - start + 1) as i64));
                    }
                }
            }
        }
        regions.extend(BOXS.iter().map(|&b| (b, 1)));

        for (region, units) in regions {
            let mut inside_sum = 0i64;
            let mut partial_cells = 0u128;
            let mut partial_sum = 0i64;
            for cage in &puzzle.cages {
                if cage.cells & !region == 0 {
                    inside_sum += cage.sum as i64;
                } else if cage.cells & region != 0 {
                    partial_cells |= cage.cells;
                    partial_sum += cage.sum as i64;
                }
            }
            let innie_sum = 45 * units - inside_sum;
            self.add_constraint(region & partial_cells, innie_sum);
            self.add_constraint(partial_cells & !region, partial_sum - innie_sum);
        }
    }

    // ヒントを置いた初期盤面。ヒント同士が矛盾していれば None。
    pub fn initial_board(&self, givens: &[usize; HW2]) -> Option<Board> {
        let mut board = new_board();
        for id in 0..HW2 {
            if givens[id] != 0 {
                let num = givens[id] - 1;
                if board.monitors[num] >> id & 1 == 0 {
                    return None;
                }
                place(&mut board, &self.peers_inverted, id, num);
            }
        }
        Some(board)
    }

    // 伝播を不動点まで回す。矛盾を見つけたら false。
    pub fn propagate(&self, board: &mut Board) -> bool {
        if self.infeasible {
            return false;
        }
        loop {
            let (invalid, hidden_none) = hidden_single(board, &self.peers_inverted);
            if invalid {
                return false;
            }
            let (invalid, naked_none) = naked_single(board, &self.peers_inverted);
            if invalid {
                return false;
            }
            let mut changed = !(hidden_none && naked_none);
            for constraint in &self.constraints {
                match constraint.apply(board, &self.peers_inverted) {
                    Err(Contradiction) => return false,
                    Ok(c) => changed |= c,
                }
            }
            if !changed {
                return true;
            }
        }
    }
}

impl SumConstraint {
    // 戻り値は候補を消したか・数字を置いたか。
    fn apply(&self, board: &mut Board, peers_inverted: &[u128; HW2]) -> Result<bool, Contradiction> {
        let mut placed: u16 = 0;
        let mut placed_sum = 0;
        for num in 0..HW {
            let m = board.numbers[num] & self.cells;
            if m != 0 {
                let n = m.count_ones();
                if self.distinct && n > 1 {
                    return Err(Contradiction);
                }
                placed |= 1 << num;
                placed_sum += (num as u32 + 1) * n;
            }
        }
        if placed_sum > self.sum {
            return Err(Contradiction);
        }
        let rem = self.sum - placed_sum;

        let empty_cells = self.cells & board.empty;
        if empty_cells == 0 {
            return if rem == 0 { Ok(false) } else { Err(Contradiction) };
        }

        let mut ids = [0usize; HW];
        let mut candidates = [0u16; HW];
        let mut k = 0;
        for id in bits(empty_cells) {
            ids[k] = id;
            candidates[k] = cell_candidates(board, id);
            if candidates[k] == 0 {
                return Err(Contradiction);
            }
            k += 1;
        }

        if self.distinct {
            self.apply_distinct(board, peers_inverted, empty_cells, placed, rem, &candidates[..k])
        } else {
            apply_bounds(board, rem, &ids[..k], &candidates[..k])
        }
    }

    fn apply_distinct(
        &self,
        board: &mut Board,
        peers_inverted: &[u128; HW2],
        empty_cells: u128,
        placed: u16,
        rem: u32,
        candidates: &[u16],
    ) -> Result<bool, Contradiction> {
        let avail = candidates.iter().fold(0, |acc, &c| acc | c) & !placed;

        // 各マスに 1 つは候補が残る組み合わせだけを成立とみなす。
        let mut allowed = 0u16;
        let mut required = 0x1ffu16;
        for &combo in combos(candidates.len(), rem) {
            if combo & !avail != 0 || candidates.iter().any(|&c| c & combo == 0) {
                continue;
            }
            allowed |= combo;
            required &= combo;
        }
        if allowed == 0 {
            return Err(Contradiction);
        }

        let mut changed = false;
        let mut remove = avail & !allowed;
        while remove != 0 {
            let num = remove.trailing_zeros() as usize;
            board.monitors[num] &= !empty_cells;
            changed = true;
            remove &= remove - 1;
        }

        // 必須の数字: 置き場所が 1 マスなら確定、1 つの単位に収まるならその単位のケージ外から消す。
        let mut required = required;
        while required != 0 {
            let num = required.trailing_zeros() as usize;
            required &= required - 1;
            let dcells = board.monitors[num] & empty_cells;
            if dcells == 0 {
                return Err(Contradiction);
            }
            if dcells & (dcells - 1) == 0 {
                place(board, peers_inverted, dcells.trailing_zeros() as usize, num);
                return Ok(true);
            }
            for unit in UNITS {
                if dcells & !unit == 0 {
                    let outside = unit & !self.cells & board.monitors[num] & board.empty;
                    if outside != 0 {
                        board.monitors[num] &= !outside;
                        changed = true;
                    }
                }
            }
        }
        Ok(changed)
    }
}

// 重複ありうる合計制約: 他マスの最小・最大から各マスの取りうる範囲を絞る。
fn apply_bounds(board: &mut Board, rem: u32, ids: &[usize], candidates: &[u16]) -> Result<bool, Contradiction> {
    let lowest = |c: u16| c.trailing_zeros() as i32 + 1;
    let highest = |c: u16| 16 - c.leading_zeros() as i32;
    let rem = rem as i32;
    let sum_min: i32 = candidates.iter().map(|&c| lowest(c)).sum();
    let sum_max: i32 = candidates.iter().map(|&c| highest(c)).sum();
    if rem < sum_min || rem > sum_max {
        return Err(Contradiction);
    }

    let mut changed = false;
    for (&id, &c) in ids.iter().zip(candidates) {
        let lo = (rem - (sum_max - highest(c))).max(1);
        let hi = (rem - (sum_min - lowest(c))).min(HW as i32);
        let keep = if lo > hi { 0 } else { ((1u16 << hi) - 1) & !((1u16 << (lo - 1)) - 1) };
        let mut remove = c & !keep;
        while remove != 0 {
            let num = remove.trailing_zeros() as usize;
            board.monitors[num] &= !(1 << id);
            changed = true;
            remove &= remove - 1;
        }
    }
    Ok(changed)
}
