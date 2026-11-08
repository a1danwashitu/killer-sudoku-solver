// 問題生成。全マス 1 マスケージ(自明に論理で解ける)から始め、隣接ケージの併合を
// ランダム順に試して「論理で解ける」ままなら採用する(nonabit の search4 の
// 「ヒントを消して一意なら採用」をケージ併合に置き換えたもの)。
use super::board::{bits, new_board};
use super::constants::*;
use super::logic::solve_logic;
use super::propagate::Context;
use super::puzzle::Puzzle;
use super::search::random_solution;

use rand::seq::SliceRandom;
use rand::Rng;

pub struct GenOptions {
    pub max_cage_size: usize,
}

impl Default for GenOptions {
    fn default() -> Self {
        Self { max_cage_size: 5 }
    }
}

pub fn generate_full<R: Rng>(rng: &mut R) -> [usize; HW2] {
    random_solution(&Context::sudoku(), new_board(), rng).expect("empty sudoku always has a solution")
}

// 隣接するマスの組(右と下)。
fn adjacent_pairs() -> Vec<(usize, usize)> {
    let mut pairs = vec![];
    for id in 0..HW2 {
        if id % HW + 1 < HW {
            pairs.push((id, id + 1));
        }
        if id + HW < HW2 {
            pairs.push((id, id + HW));
        }
    }
    pairs
}

fn digits_of(cells: u128, answer: &[usize; HW2]) -> u16 {
    bits(cells).fold(0, |acc, id| acc | 1 << answer[id])
}

pub fn generate<R: Rng>(rng: &mut R, options: &GenOptions) -> Puzzle {
    let answer = generate_full(rng);
    let mut cages: Vec<u128> = (0..HW2).map(|id| 1 << id).collect();
    let mut cage_of: [usize; HW2] = std::array::from_fn(|id| id);
    let mut pairs = adjacent_pairs();

    // 1 パスで 1 つも採用されなくなるまで繰り返す。
    loop {
        pairs.shuffle(rng);
        let mut merged_any = false;
        for &(a, b) in &pairs {
            let (ia, ib) = (cage_of[a], cage_of[b]);
            if ia == ib {
                continue;
            }
            let merged = cages[ia] | cages[ib];
            if merged.count_ones() as usize > options.max_cage_size
                || digits_of(cages[ia], &answer) & digits_of(cages[ib], &answer) != 0
            {
                continue;
            }

            let candidate: Vec<u128> = cages
                .iter()
                .enumerate()
                .filter(|&(i, &cells)| i != ib && cells != 0)
                .map(|(i, &cells)| if i == ia { merged } else { cells })
                .collect();
            if !solve_logic(&Puzzle::from_answer(&candidate, &answer)).solved {
                continue;
            }

            for id in bits(cages[ib]) {
                cage_of[id] = ia;
            }
            cages[ia] = merged;
            cages[ib] = 0;
            merged_any = true;
        }
        if !merged_any {
            break;
        }
    }

    let cells: Vec<u128> = cages.into_iter().filter(|&cells| cells != 0).collect();
    Puzzle::from_answer(&cells, &answer)
}
