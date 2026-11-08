// 伝播 + 分岐の DFS。sudoku-nonabit/src/solver1.rs の dfs_ans_counts / dfs_random を流用し、
// 確定処理を Context::propagate(ケージ制約込み)に置き換えたもの。
use super::board::*;
use super::constants::*;
use super::propagate::Context;
use super::puzzle::Puzzle;

use rand::seq::SliceRandom;

#[inline]
fn full_adder(a: u128, b: u128, c: u128) -> (u128, u128) {
    (a ^ b ^ c, (a & b) | (b & c) | (a & c))
}

#[inline]
fn half_adder(a: u128, b: u128) -> (u128, u128) {
    (a ^ b, a & b)
}

// 9 枚の monitors 平面を全加算器ネットワークに通し、セルごとの残り候補数を
// 4 本のビット平面として並列に求める(nonabit と同じ)。
fn candidate_count_planes(monitors: &[u128; HW]) -> (u128, u128, u128, u128) {
    let (s1, c1) = full_adder(monitors[0], monitors[1], monitors[2]);
    let (s2, c2) = full_adder(monitors[3], monitors[4], monitors[5]);
    let (s3, c3) = full_adder(monitors[6], monitors[7], monitors[8]);

    let (bit0, carry_a) = full_adder(s1, s2, s3);
    let (sum_b, carry_b) = full_adder(c1, c2, c3);

    let (bit1, carry_c) = half_adder(carry_a, sum_b);
    let (bit2, bit3) = half_adder(carry_b, carry_c);

    (bit0, bit1, bit2, bit3)
}

// 残り候補数が最小の空きマス(同点なら最小インデックス)。
fn select_mrv_cell(board: &Board) -> usize {
    let (bit0, bit1, bit2, bit3) = candidate_count_planes(&board.monitors);
    let planes = [bit0, bit1, bit2, bit3];

    for count in 0..=HW {
        let mut mask = board.empty;
        for (k, &plane) in planes.iter().enumerate() {
            mask &= if count >> k & 1 == 1 { plane } else { !plane };
        }
        if mask != 0 {
            return mask.trailing_zeros() as usize;
        }
    }

    board.empty.trailing_zeros() as usize
}

pub struct SearchResult {
    // max_count で打ち切った解の個数。
    pub count: usize,
    pub solution: Option<[usize; HW2]>,
    pub nodes: usize,
}

// 解を max_count 個まで数える。一意性判定は max_count = 2。
pub fn count_solutions(context: &Context, board: Board, max_count: usize) -> SearchResult {
    let mut result = SearchResult {
        count: 0,
        solution: None,
        nodes: 0,
    };
    let mut stack = vec![board];

    while let Some(mut board) = stack.pop() {
        result.nodes += 1;
        if !context.propagate(&mut board) {
            continue;
        }
        if board.empty == 0 {
            result.count += 1;
            if result.solution.is_none() {
                result.solution = Some(bitboard_to_array(board.numbers));
            }
            if result.count >= max_count {
                break;
            }
            continue;
        }

        let id = select_mrv_cell(&board);
        for num in 0..HW {
            if board.monitors[num] >> id & 1 == 1 {
                let mut new_board = board.clone();
                place(&mut new_board, &context.peers_inverted, id, num);
                stack.push(new_board);
            }
        }
    }
    result
}

pub fn solve_puzzle(puzzle: &Puzzle, max_count: usize) -> SearchResult {
    let context = Context::new(puzzle, true);
    match context.initial_board(&puzzle.givens) {
        Some(board) => count_solutions(&context, board, max_count),
        None => SearchResult {
            count: 0,
            solution: None,
            nodes: 0,
        },
    }
}

// 分岐順をシャッフルして最初に見つかった解を返す(ランダムな完成盤面の生成用)。
pub fn random_solution<R: rand::Rng>(context: &Context, board: Board, rng: &mut R) -> Option<[usize; HW2]> {
    let mut stack = vec![board];

    while let Some(mut board) = stack.pop() {
        if !context.propagate(&mut board) {
            continue;
        }
        if board.empty == 0 {
            return Some(bitboard_to_array(board.numbers));
        }

        let id = board.empty.trailing_zeros() as usize;
        let mut numbers = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        numbers.shuffle(rng);
        for num in numbers {
            if board.monitors[num] >> id & 1 == 1 {
                let mut new_board = board.clone();
                place(&mut new_board, &context.peers_inverted, id, num);
                stack.push(new_board);
            }
        }
    }
    None
}
