use std::io::{self, Read};
use std::time::Instant;

use killer_sudoku_solver::display::render;
use killer_sudoku_solver::generate::{generate, GenOptions};
use killer_sudoku_solver::logic::solve_logic;
use killer_sudoku_solver::puzzle::Puzzle;
use killer_sudoku_solver::search::solve_puzzle;

use rand::rngs::StdRng;
use rand::SeedableRng;

const USAGE: &str = "usage:
  killer-sudoku-solver generate [-n COUNT] [--seed SEED] [--max-size SIZE] [--quiet]
  killer-sudoku-solver solve [FILE]    (FILE 省略時は標準入力。1 行 1 問、# 始まりはコメント)
  killer-sudoku-solver html [-n COUNT] [--seed SEED] [--max-size SIZE] [--from FILE] [--wasm FILE] [-o OUT]
      問題入りのブラウザ用ページを書き出す(既定 -n 20、-o play.html)。
      --from を指定するとファイルの問題を使い、生成はしない。
      --wasm で wasm ビルドの生成器を埋め込むと、ページ上で新しい問題を生成できる。";

// ブラウザで解くためのプレイヤー。`/*PUZZLES*/[]/*END*/` を問題の配列に、
// `/*WASM*/""/*END*/` を生成器(wasm)の base64 文字列に置き換えて書き出す。
const PLAYER_TEMPLATE: &str = include_str!("../web/player.html");
const PUZZLES_PLACEHOLDER: &str = "/*PUZZLES*/[]/*END*/";
const WASM_PLACEHOLDER: &str = "/*WASM*/\"\"/*END*/";

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("generate") => cmd_generate(&args[1..]),
        Some("solve") => cmd_solve(&args[1..]),
        Some("html") => cmd_html(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(message) = result {
        eprintln!("{}", message);
        std::process::exit(1);
    }
}

fn parse_value<T: std::str::FromStr>(args: &[String], i: usize) -> Result<T, String> {
    args.get(i)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("invalid value for {}\n{}", args[i - 1], USAGE))
}

fn cmd_generate(args: &[String]) -> Result<(), String> {
    let mut count = 1;
    let mut seed: Option<u64> = None;
    let mut options = GenOptions::default();
    let mut quiet = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                count = parse_value(args, i)?;
            }
            "--seed" => {
                i += 1;
                seed = Some(parse_value(args, i)?);
            }
            "--max-size" => {
                i += 1;
                options.max_cage_size = parse_value(args, i)?;
            }
            "--quiet" => quiet = true,
            other => return Err(format!("unknown option {}\n{}", other, USAGE)),
        }
        i += 1;
    }

    let mut rng = match seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    };
    for _ in 0..count {
        let start = Instant::now();
        let puzzle = generate(&mut rng, &options);
        let elapsed = start.elapsed();
        if quiet {
            println!("{}", puzzle.to_line());
            continue;
        }
        let mut sizes = [0; 10];
        for cage in &puzzle.cages {
            sizes[cage.cells.count_ones() as usize] += 1;
        }
        print!("{}", render(&puzzle, &puzzle.givens));
        println!("cages: {}  sizes(1..9): {:?}  time: {:?}", puzzle.cages.len(), &sizes[1..], elapsed);
        println!("{}", puzzle.to_line());
        println!();
    }
    Ok(())
}

fn read_puzzle_lines(input: &str) -> Result<Vec<Puzzle>, String> {
    input
        .lines()
        .enumerate()
        .map(|(lineno, line)| (lineno, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .map(|(lineno, line)| {
            Puzzle::parse(line)
                .and_then(|p| p.validate().map(|_| p))
                .map_err(|e| format!("line {}: invalid puzzle: {}", lineno + 1, e))
        })
        .collect()
}

// JS の文字列リテラルとして安全に埋め込む。`<` は </script> 対策でエスケープする。
fn js_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn cmd_html(args: &[String]) -> Result<(), String> {
    let mut count = 20;
    let mut seed: Option<u64> = None;
    let mut options = GenOptions::default();
    let mut from: Option<String> = None;
    let mut wasm: Option<String> = None;
    let mut out = String::from("play.html");
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                count = parse_value(args, i)?;
            }
            "--seed" => {
                i += 1;
                seed = Some(parse_value(args, i)?);
            }
            "--max-size" => {
                i += 1;
                options.max_cage_size = parse_value(args, i)?;
            }
            "--from" => {
                i += 1;
                from = Some(parse_value(args, i)?);
            }
            "--wasm" => {
                i += 1;
                wasm = Some(parse_value(args, i)?);
            }
            "-o" => {
                i += 1;
                out = parse_value(args, i)?;
            }
            other => return Err(format!("unknown option {}\n{}", other, USAGE)),
        }
        i += 1;
    }

    let puzzles = match from {
        Some(path) => {
            let input = std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path, e))?;
            read_puzzle_lines(&input)?
        }
        None => {
            let mut rng = match seed {
                Some(seed) => StdRng::seed_from_u64(seed),
                None => StdRng::from_entropy(),
            };
            (0..count).map(|_| generate(&mut rng, &options)).collect()
        }
    };

    let lines: Vec<String> = puzzles.iter().map(|p| js_string(&p.to_line())).collect();
    let embedded = format!("[\n{}\n]", lines.join(",\n"));
    if !PLAYER_TEMPLATE.contains(PUZZLES_PLACEHOLDER) || !PLAYER_TEMPLATE.contains(WASM_PLACEHOLDER) {
        return Err("player template has no placeholder".to_string());
    }
    let mut html = PLAYER_TEMPLATE.replacen(PUZZLES_PLACEHOLDER, &embedded, 1);
    if let Some(path) = &wasm {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {}", path, e))?;
        html = html.replacen(WASM_PLACEHOLDER, &format!("\"{}\"", base64(&bytes)), 1);
    }
    std::fs::write(&out, html).map_err(|e| format!("{}: {}", out, e))?;
    println!(
        "{} 問を {} に書き出しました{}",
        puzzles.len(),
        out,
        if wasm.is_some() { "(生成器入り)" } else { "" }
    );
    Ok(())
}

fn cmd_solve(args: &[String]) -> Result<(), String> {
    let mut input = String::new();
    match args.first() {
        Some(path) => input = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?,
        None => {
            io::stdin().read_to_string(&mut input).map_err(|e| e.to_string())?;
        }
    }

    for (lineno, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let puzzle = match Puzzle::parse(line).and_then(|p| p.validate().map(|_| p)) {
            Ok(p) => p,
            Err(e) => {
                println!("line {}: invalid puzzle: {}\n", lineno + 1, e);
                continue;
            }
        };

        let start = Instant::now();
        let logic = solve_logic(&puzzle);
        let logic_time = start.elapsed();
        let start = Instant::now();
        let search = solve_puzzle(&puzzle, 2);
        let search_time = start.elapsed();

        let grid = search.solution.unwrap_or(logic.grid);
        print!("{}", render(&puzzle, &grid));
        println!(
            "logic: {} ({:?})  search: {} ({} nodes, {:?})",
            if logic.solved { "solved" } else { "stuck" },
            logic_time,
            match search.count {
                0 => "no solution",
                1 => "unique",
                _ => "multiple solutions",
            },
            search.nodes,
            search_time,
        );
        if let (Some(answer), Some(solution)) = (&puzzle.answer, &search.solution) {
            if answer != solution {
                println!("warning: solution differs from the answer in the input");
            }
        }
        println!();
    }
    Ok(())
}
