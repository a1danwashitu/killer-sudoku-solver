// テキスト表示。ケージ境界は - と |、ケージ境界でない箱の境界は . と : で描く。
// 各ケージの先頭マス(左上寄り)に合計値を書く。全角幅で崩れないよう ASCII のみ使う。
use super::constants::*;
use super::puzzle::Puzzle;

const CELL_WIDTH: usize = 4;

pub fn render(puzzle: &Puzzle, grid: &[usize; HW2]) -> String {
    let ids = puzzle.cage_ids();
    let cage_at = |r: usize, c: usize| ids[r * HW + c];

    // (r, c) のマスの上辺。
    let hseg = |r: usize, c: usize| -> char {
        if r == 0 || r == HW || cage_at(r - 1, c) != cage_at(r, c) {
            '-'
        } else if r % HWRT == 0 {
            '.'
        } else {
            ' '
        }
    };
    // (r, c) のマスの左辺。
    let vseg = |r: usize, c: usize| -> char {
        if c == 0 || c == HW || cage_at(r, c - 1) != cage_at(r, c) {
            '|'
        } else if c % HWRT == 0 {
            ':'
        } else {
            ' '
        }
    };
    let corner = |r: usize, c: usize| -> char {
        let mut around = vec![];
        if c > 0 {
            around.push(hseg(r, c - 1));
        }
        if c < HW {
            around.push(hseg(r, c));
        }
        if r > 0 {
            around.push(vseg(r - 1, c));
        }
        if r < HW {
            around.push(vseg(r, c));
        }
        if around.iter().any(|&ch| ch == '-' || ch == '|') {
            '+'
        } else if around.iter().any(|&ch| ch == '.' || ch == ':') {
            '.'
        } else {
            ' '
        }
    };

    let mut out = String::new();
    for r in 0..=HW {
        for c in 0..=HW {
            out.push(corner(r, c));
            if c < HW {
                out.push_str(&hseg(r, c).to_string().repeat(CELL_WIDTH));
            }
        }
        out.push('\n');
        if r == HW {
            break;
        }
        for c in 0..=HW {
            out.push(vseg(r, c));
            if c < HW {
                let id = r * HW + c;
                let cage = &puzzle.cages[ids[id]];
                let label = if cage.cells.trailing_zeros() as usize == id {
                    cage.sum.to_string()
                } else {
                    String::new()
                };
                let value = if grid[id] == 0 { ' ' } else { char::from(b'0' + grid[id] as u8) };
                out.push_str(&format!("{:<2} {}", label, value));
            }
        }
        out.push('\n');
    }
    out
}
