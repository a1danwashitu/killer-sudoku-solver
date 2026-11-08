use super::board::bits;
use super::combos::sum_range;
use super::constants::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cage {
    pub cells: u128,
    pub sum: u32,
}

#[derive(Clone, Debug)]
pub struct Puzzle {
    // 先頭マス(最小インデックス)の昇順に並べる。データ形式のラベル順と一致させるため。
    pub cages: Vec<Cage>,
    pub givens: [usize; HW2],
    pub answer: Option<[usize; HW2]>,
}

// 出力時のケージラベル。区切り文字(; ,)と空きマス記号(.)は使わない。81 個以上必要。
const LABELS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+-<=>?@[]^_{|}~/";

// 上下左右に隣接するマスへ広げたマスク。
pub fn neighbors(mask: u128) -> u128 {
    (((mask << 1) & !COLS[0]) | ((mask >> 1) & !COLS[HW - 1]) | (mask << HW) | (mask >> HW))
        & BOARD_ALL
}

pub fn is_connected(mask: u128) -> bool {
    if mask == 0 {
        return false;
    }
    let mut reached = mask & mask.wrapping_neg();
    loop {
        let next = (reached | neighbors(reached)) & mask;
        if next == reached {
            return reached == mask;
        }
        reached = next;
    }
}

fn parse_grid(field: &str) -> Result<[usize; HW2], String> {
    let chars: Vec<char> = field.trim().chars().collect();
    if chars.len() != HW2 {
        return Err(format!("grid must have 81 chars, got {}", chars.len()));
    }
    let mut grid = [0; HW2];
    for (id, ch) in chars.into_iter().enumerate() {
        grid[id] = match ch {
            '1'..='9' => ch.to_digit(10).unwrap() as usize,
            '.' | '0' => 0,
            _ => return Err(format!("invalid grid char '{}'", ch)),
        };
    }
    Ok(grid)
}

fn format_grid(grid: &[usize; HW2]) -> String {
    grid.iter()
        .map(|&n| if n == 0 { '.' } else { char::from(b'0' + n as u8) })
        .collect()
}

impl Puzzle {
    pub fn new(mut cages: Vec<Cage>, givens: [usize; HW2]) -> Self {
        cages.sort_by_key(|cage| cage.cells.trailing_zeros());
        Self {
            cages,
            givens,
            answer: None,
        }
    }

    // 解答とケージ分割から、合計値を解答から計算して問題を作る。
    pub fn from_answer(cage_cells: &[u128], answer: &[usize; HW2]) -> Self {
        let cages = cage_cells
            .iter()
            .map(|&cells| Cage {
                cells,
                sum: bits(cells).map(|id| answer[id] as u32).sum(),
            })
            .collect();
        let mut puzzle = Self::new(cages, [0; HW2]);
        puzzle.answer = Some(*answer);
        puzzle
    }

    // 各マスが属するケージの番号(self.cages の添字)。
    pub fn cage_ids(&self) -> [usize; HW2] {
        let mut ids = [usize::MAX; HW2];
        for (i, cage) in self.cages.iter().enumerate() {
            for id in bits(cage.cells) {
                ids[id] = i;
            }
        }
        ids
    }

    // `<ケージラベル 81 文字>;<合計値カンマ区切り>[;<ヒント>[;<解答>]]`
    pub fn parse(line: &str) -> Result<Self, String> {
        let fields: Vec<&str> = line.trim().split(';').collect();
        if fields.len() < 2 || fields.len() > 4 {
            return Err(format!("expected 2-4 fields, got {}", fields.len()));
        }

        let labels: Vec<char> = fields[0].trim().chars().collect();
        if labels.len() != HW2 {
            return Err(format!("cage labels must have 81 chars, got {}", labels.len()));
        }
        // ラベルは初出順に番号を振る(= 先頭マスの昇順)。
        let mut order: Vec<char> = vec![];
        let mut cells: Vec<u128> = vec![];
        for (id, &label) in labels.iter().enumerate() {
            let i = match order.iter().position(|&l| l == label) {
                Some(i) => i,
                None => {
                    order.push(label);
                    cells.push(0);
                    order.len() - 1
                }
            };
            cells[i] |= 1 << id;
        }

        let sums: Vec<u32> = fields[1]
            .split(',')
            .map(|s| s.trim().parse::<u32>().map_err(|e| format!("invalid sum '{}': {}", s, e)))
            .collect::<Result<_, _>>()?;
        if sums.len() != cells.len() {
            return Err(format!("{} cages but {} sums", cells.len(), sums.len()));
        }

        let cages = cells
            .into_iter()
            .zip(sums)
            .map(|(cells, sum)| Cage { cells, sum })
            .collect();
        let givens = match fields.get(2) {
            Some(field) => parse_grid(field)?,
            None => [0; HW2],
        };
        let mut puzzle = Self::new(cages, givens);
        if let Some(field) = fields.get(3) {
            puzzle.answer = Some(parse_grid(field)?);
        }
        Ok(puzzle)
    }

    pub fn to_line(&self) -> String {
        let ids = self.cage_ids();
        let labels: String = ids.iter().map(|&i| char::from(LABELS[i])).collect();
        let sums: Vec<String> = self.cages.iter().map(|cage| cage.sum.to_string()).collect();
        let mut line = format!("{};{}", labels, sums.join(","));
        if self.givens.iter().any(|&n| n != 0) || self.answer.is_some() {
            line.push(';');
            line.push_str(&format_grid(&self.givens));
        }
        if let Some(answer) = &self.answer {
            line.push(';');
            line.push_str(&format_grid(answer));
        }
        line
    }

    // 盤面の構造的な検証。ヒント同士の矛盾はソルバー側(Context::initial_board)で弾く。
    pub fn validate(&self) -> Result<(), String> {
        let mut covered = 0u128;
        let mut total = 0;
        for cage in &self.cages {
            if cage.cells == 0 || cage.cells & !BOARD_ALL != 0 {
                return Err("cage has no cells or cells out of board".to_string());
            }
            if cage.cells & covered != 0 {
                return Err("cages overlap".to_string());
            }
            covered |= cage.cells;
            let size = cage.cells.count_ones() as usize;
            if size > HW {
                return Err(format!("cage size {} exceeds 9", size));
            }
            if !is_connected(cage.cells) {
                return Err("cage is not connected".to_string());
            }
            let (min, max) = sum_range(size);
            if cage.sum < min || cage.sum > max {
                return Err(format!("sum {} impossible for cage of size {}", cage.sum, size));
            }
            total += cage.sum;
        }
        if covered != BOARD_ALL {
            return Err("cages do not cover the whole board".to_string());
        }
        if total != 405 {
            return Err(format!("total of sums must be 405, got {}", total));
        }
        Ok(())
    }

    // grid がこの問題の正しい解かを、ソルバーとは独立に素朴に確かめる。
    pub fn is_solution(&self, grid: &[usize; HW2]) -> bool {
        let mut units: Vec<u128> = UNITS.to_vec();
        units.extend(self.cages.iter().map(|cage| cage.cells));
        for unit in units {
            let mut seen = 0u16;
            for id in bits(unit) {
                let n = grid[id];
                if n == 0 || n > HW || seen >> n & 1 == 1 {
                    return false;
                }
                seen |= 1 << n;
            }
        }
        let sums_ok = self
            .cages
            .iter()
            .all(|cage| bits(cage.cells).map(|id| grid[id] as u32).sum::<u32>() == cage.sum);
        let givens_ok = (0..HW2).all(|id| self.givens[id] == 0 || self.givens[id] == grid[id]);
        sums_ok && givens_ok
    }
}
