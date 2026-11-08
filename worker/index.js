// クリアタイムのランキング API。静的ファイル(dist/)に無いパスだけがここに来る。
//   POST /api/records  { name, category, puzzle, values, seconds } → { ok, date, rankMonth, rankAll }
//   GET  /api/ranking?category=5&period=all|YYYY-MM[&limit=50] → { records: [{ name, seconds, date }] }

const CATEGORIES = [4, 5, 6, 9];
const NAME_MAX = 20;
const MIN_SECONDS = 10;
const MAX_SECONDS = 24 * 3600;

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    try {
      if (url.pathname === '/api/records' && request.method === 'POST') return await postRecord(request, env);
      if (url.pathname === '/api/ranking' && request.method === 'GET') return await getRanking(url, env);
      if (url.pathname.startsWith('/api/')) return json({ error: 'not found' }, 404);
      return env.ASSETS.fetch(request);
    } catch (err) {
      if (err instanceof BadRequest) return json({ error: err.message }, 400);
      console.error(err);
      return json({ error: 'サーバーでエラーが発生しました' }, 500);
    }
  },
};

class BadRequest extends Error {}

function json(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' },
  });
}

// 日本時間の YYYY-MM-DD
function jstDate(now = Date.now()) {
  return new Date(now + 9 * 3600 * 1000).toISOString().slice(0, 10);
}

function cleanName(raw) {
  if (typeof raw !== 'string') throw new BadRequest('名前がありません');
  // 制御文字を除き、連続する空白を 1 つにする
  const name = raw.replace(/[\u0000-\u001f\u007f]/g, '').replace(/\s+/g, ' ').trim();
  if (!name) throw new BadRequest('名前を入力してください');
  if (Array.from(name).length > NAME_MAX) throw new BadRequest(`名前は${NAME_MAX}文字までです`);
  return name;
}

// 問題(`ケージラベル;合計値[;ヒント[;解答]]`)と解答(81 桁)を検証し、
// 問題のキー「ケージラベル;合計値」を返す。ヒント数字のある問題は受け付けない。
function checkSolution(puzzle, values, category) {
  if (typeof puzzle !== 'string' || typeof values !== 'string') throw new BadRequest('データが不正です');
  const fields = puzzle.trim().split(';');
  if (fields.length < 2 || fields.length > 4) throw new BadRequest('問題の形式が不正です');
  const labels = Array.from(fields[0].trim());
  if (labels.length !== 81) throw new BadRequest('問題の形式が不正です');
  if (fields[2] && /[1-9]/.test(fields[2])) throw new BadRequest('ヒント数字のある問題は記録できません');
  if (!/^[1-9]{81}$/.test(values)) throw new BadRequest('解答が不正です');
  const v = Array.from(values, Number);

  const cages = new Map();
  labels.forEach((label, id) => {
    if (!cages.has(label)) cages.set(label, []);
    cages.get(label).push(id);
  });
  const sums = fields[1].split(',').map((s) => Number(s.trim()));
  if (sums.length !== cages.size || sums.some((s) => !Number.isInteger(s))) throw new BadRequest('問題の形式が不正です');

  const distinct = (ids) => new Set(ids.map((id) => v[id])).size === ids.length;
  for (let i = 0; i < 9; i++) {
    const row = [], col = [], box = [];
    for (let j = 0; j < 9; j++) {
      row.push(i * 9 + j);
      col.push(j * 9 + i);
      box.push((Math.floor(i / 3) * 3 + Math.floor(j / 3)) * 9 + (i % 3) * 3 + (j % 3));
    }
    if (!distinct(row) || !distinct(col) || !distinct(box)) throw new BadRequest('解答が正しくありません');
  }
  let maxSize = 0;
  [...cages.values()].forEach((ids, i) => {
    maxSize = Math.max(maxSize, ids.length);
    if (!distinct(ids) || ids.reduce((a, id) => a + v[id], 0) !== sums[i]) throw new BadRequest('解答が正しくありません');
  });
  if (maxSize > category) throw new BadRequest('ケージの大きさが区分と合いません');
  return fields[0].trim() + ';' + fields[1].trim();
}

async function postRecord(request, env) {
  let body;
  try { body = await request.json(); } catch { throw new BadRequest('データが不正です'); }
  const name = cleanName(body.name);
  const category = Number(body.category);
  if (!CATEGORIES.includes(category)) throw new BadRequest('区分が不正です');
  const seconds = Number(body.seconds);
  if (!Number.isInteger(seconds) || seconds < MIN_SECONDS || seconds > MAX_SECONDS) throw new BadRequest('タイムが不正です');
  const key = checkSolution(body.puzzle, body.values, category);

  const now = Date.now();
  const date = jstDate(now);
  const month = date.slice(0, 7);
  const result = await env.DB.prepare(
    `INSERT INTO records (name, seconds, category, puzzle, month, date, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT (puzzle, name) DO NOTHING`,
  ).bind(name, seconds, category, key, month, date, new Date(now).toISOString()).run();
  if (!result.meta.changes) throw new BadRequest('この問題はその名前ですでに記録されています');

  // 同タイムは先に登録した人が上
  const id = result.meta.last_row_id;
  const [all, monthly] = await env.DB.batch([
    env.DB.prepare('SELECT COUNT(*) AS n FROM records WHERE category = ? AND (seconds < ? OR (seconds = ? AND id < ?))')
      .bind(category, seconds, seconds, id),
    env.DB.prepare('SELECT COUNT(*) AS n FROM records WHERE category = ? AND month = ? AND (seconds < ? OR (seconds = ? AND id < ?))')
      .bind(category, month, seconds, seconds, id),
  ]);
  return json({ ok: true, date, month, rankAll: all.results[0].n + 1, rankMonth: monthly.results[0].n + 1 });
}

async function getRanking(url, env) {
  const category = Number(url.searchParams.get('category'));
  if (!CATEGORIES.includes(category)) throw new BadRequest('区分が不正です');
  const period = url.searchParams.get('period') || 'all';
  if (period !== 'all' && !/^\d{4}-\d{2}$/.test(period)) throw new BadRequest('期間が不正です');
  const limit = Math.min(Math.max(Number(url.searchParams.get('limit')) || 50, 1), 100);
  const stmt = period === 'all'
    ? env.DB.prepare('SELECT name, seconds, date FROM records WHERE category = ? ORDER BY seconds, id LIMIT ?').bind(category, limit)
    : env.DB.prepare('SELECT name, seconds, date FROM records WHERE category = ? AND month = ? ORDER BY seconds, id LIMIT ?').bind(category, period, limit);
  const { results } = await stmt.all();
  return json({ records: results });
}
