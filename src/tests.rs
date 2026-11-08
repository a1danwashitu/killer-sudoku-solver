use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::board::bits;
use crate::brute;
use crate::combos::combos;
use crate::constants::*;
use crate::generate::{generate, generate_full, GenOptions};
use crate::logic::solve_logic;
use crate::propagate::Context;
use crate::puzzle::{neighbors, Cage, Puzzle};
use crate::search::{count_solutions, solve_puzzle};

const TEST_SEED: u64 = 20261029;

fn mask_of(digits: &[usize]) -> u16 {
    digits.iter().fold(0, |acc, &d| acc | 1 << (d - 1))
}

// 数字の重複を気にせずランダムに盤面をケージ分割する(照合テスト用)。
fn random_partition<R: Rng>(rng: &mut R, max_size: usize) -> Vec<u128> {
    let mut free = BOARD_ALL;
    let mut cages = vec![];
    while free != 0 {
        let ids: Vec<usize> = bits(free).collect();
        let start = *ids.choose(rng).unwrap();
        let mut cage = 1u128 << start;
        let target = rng.gen_range(1..=max_size);
        while (cage.count_ones() as usize) < target {
            let frontier: Vec<usize> = bits(neighbors(cage) & free & !cage).collect();
            match frontier.choose(rng) {
                Some(&id) => cage |= 1 << id,
                None => break,
            }
        }
        free &= !cage;
        cages.push(cage);
    }
    cages
}

#[test]
fn combos_table() {
    assert_eq!(combos(2, 3), &[mask_of(&[1, 2])]);
    assert_eq!(combos(2, 17), &[mask_of(&[8, 9])]);
    assert_eq!(combos(3, 23), &[mask_of(&[6, 8, 9])]);
    assert_eq!(combos(9, 45), &[0x1ff]);
    assert_eq!(combos(2, 2), &[] as &[u16]);
    let total: usize = (0..=HW).map(|k| (0..=45).map(|s| combos(k, s).len()).sum::<usize>()).sum();
    assert_eq!(total, 512);
}

#[test]
fn parse_and_format_roundtrip() {
    let mut rng = StdRng::seed_from_u64(TEST_SEED);
    let answer = generate_full(&mut rng);
    let puzzle = Puzzle::from_answer(&random_partition(&mut rng, 4), &answer);
    let line = puzzle.to_line();
    let parsed = Puzzle::parse(&line).unwrap();
    assert_eq!(parsed.cages, puzzle.cages);
    assert_eq!(parsed.answer, Some(answer));
    assert_eq!(parsed.to_line(), line);
    assert!(parsed.validate().is_ok());
}

#[test]
fn validate_rejects_broken_puzzles() {
    let mut rng = StdRng::seed_from_u64(TEST_SEED);
    let answer = generate_full(&mut rng);
    let good = Puzzle::from_answer(&random_partition(&mut rng, 4), &answer);
    assert!(good.validate().is_ok());

    let mut wrong_total = good.clone();
    let i = wrong_total.cages.iter().position(|c| c.cells.count_ones() >= 2).unwrap();
    wrong_total.cages[i].sum -= 1;
    assert!(wrong_total.validate().is_err());

    let mut missing = good.clone();
    missing.cages.pop();
    assert!(missing.validate().is_err());

    // 角の 2 マス(0 と 80)だけのケージは連結でない。
    let mut cells: Vec<u128> = (1..HW2 - 1).map(|id| 1 << id).collect();
    cells.push(1 | 1 << (HW2 - 1));
    assert!(Puzzle::from_answer(&cells, &answer).validate().is_err());

    let mut impossible = Puzzle::from_answer(&(0..HW2).map(|id| 1 << id).collect::<Vec<_>>(), &answer);
    impossible.cages[0].sum = 10;
    assert!(impossible.validate().is_err());
}

#[test]
fn conflicting_givens_are_rejected() {
    let puzzle = Puzzle::new(
        (0..HW2).map(|id| Cage { cells: 1 << id, sum: 5 }).collect(),
        {
            let mut givens = [0; HW2];
            givens[0] = 3;
            givens[1] = 3;
            givens
        },
    );
    assert!(Context::new(&puzzle, true).initial_board(&puzzle.givens).is_none());
    assert_eq!(brute::count_solutions(&puzzle, 2).0, 0);
}

// ランダムなケージ分割(数字の重複を許すので解なし・複数解も混ざる)+ ヒントで、
// 伝播+探索の解の個数が総当たりと一致することを確かめる。45 の法則の有無の両方で照合する。
#[test]
fn search_matches_brute_force() {
    let mut rng = StdRng::seed_from_u64(TEST_SEED);
    let mut counts = [0; 4];
    for _ in 0..300 {
        let answer = generate_full(&mut rng);
        let mut puzzle = Puzzle::from_answer(&random_partition(&mut rng, 5), &answer);
        let mut ids: Vec<usize> = (0..HW2).collect();
        ids.shuffle(&mut rng);
        for &id in &ids[..rng.gen_range(25..=45)] {
            puzzle.givens[id] = answer[id];
        }
        // 一部の問題はヒントを 1 つ壊して矛盾を混ぜる。
        if rng.gen_bool(0.1) {
            let id = ids[0];
            puzzle.givens[id] = puzzle.givens[id] % 9 + 1;
        }

        let (expected, _) = brute::count_solutions(&puzzle, 3);
        for rule45 in [false, true] {
            let context = Context::new(&puzzle, rule45);
            let result = match context.initial_board(&puzzle.givens) {
                Some(board) => count_solutions(&context, board, 3),
                None => {
                    assert_eq!(expected, 0);
                    continue;
                }
            };
            assert_eq!(result.count, expected, "rule45={} {}", rule45, puzzle.to_line());
            if let Some(solution) = result.solution {
                assert!(puzzle.is_solution(&solution));
            }
        }
        counts[expected] += 1;
    }
    // 0 解・1 解・複数解がそれぞれ出ていないと照合の意味が薄い。
    assert!(counts[0] > 0 && counts[1] > 0 && counts[2] + counts[3] > 0, "{:?}", counts);
}

// 生成した問題は検証を通り、論理で解け、その解が元の解答と一致し、総当たりでも一意。
#[test]
fn generated_puzzles_are_logic_solvable_and_unique() {
    let mut rng = StdRng::seed_from_u64(TEST_SEED);
    for _ in 0..5 {
        let puzzle = generate(&mut rng, &GenOptions::default());
        assert!(puzzle.validate().is_ok());
        assert!(puzzle.givens.iter().all(|&n| n == 0));
        let answer = puzzle.answer.unwrap();
        assert!(puzzle.is_solution(&answer));

        let logic = solve_logic(&puzzle);
        assert!(logic.solved);
        assert_eq!(logic.grid, answer);

        assert_eq!(solve_puzzle(&puzzle, 2).count, 1);
        let (count, solution) = brute::count_solutions(&puzzle, 2);
        assert_eq!(count, 1);
        assert_eq!(solution, Some(answer));
    }
}

#[test]
fn generate_full_is_valid_sudoku() {
    let mut rng = StdRng::seed_from_u64(TEST_SEED);
    for _ in 0..50 {
        let grid = generate_full(&mut rng);
        for unit in UNITS {
            let seen = bits(unit).fold(0u16, |acc, id| acc | 1 << grid[id]);
            assert_eq!(seen, 0x3fe);
        }
    }
}
