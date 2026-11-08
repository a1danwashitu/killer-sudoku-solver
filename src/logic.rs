// 論理ソルバー: 分岐せず伝播だけで解く。解ければ解は一意。
use super::board::bitboard_to_array;
use super::constants::*;
use super::propagate::Context;
use super::puzzle::Puzzle;

pub struct LogicResult {
    pub solved: bool,
    // 伝播で確定したマス(未確定は 0)。
    pub grid: [usize; HW2],
}

pub fn solve_logic(puzzle: &Puzzle) -> LogicResult {
    let context = Context::new(puzzle, true);
    let Some(mut board) = context.initial_board(&puzzle.givens) else {
        return LogicResult {
            solved: false,
            grid: puzzle.givens,
        };
    };
    let consistent = context.propagate(&mut board);
    LogicResult {
        solved: consistent && board.empty == 0,
        grid: bitboard_to_array(board.numbers),
    }
}
