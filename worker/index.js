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
      if (url.pathname === '/api/admin/notify-test' && request.method === 'POST') return await notifyTest(request, env);
      if (url.pathname.startsWith('/api/')) return json({ error: 'not found' }, 404);
      return env.ASSETS.fetch(request);
    } catch (err) {
      if (err instanceof BadRequest) return json({ error: err.message }, 400);
      console.error(err);
      return json({ error: 'サーバーでエラーが発生しました' }, 500);
    }
  },

  // Cron Trigger(毎日)。月が変わったら前月の月間ランキングを Discord に通知する。
  async scheduled(controller, env, ctx) {
    ctx.waitUntil(notifyMonthly(env, controller.scheduledTime));
  },
};

// ---------- Discord への月間ランキング通知 ----------
// 毎日動かし、日本時間で月初から NOTIFY_DAYS 日以内に前月分が未送信なら送る
// (Discord が失敗しても翌日に再送される)。送った月は notifications 表に記録する。

const NOTIFY_DAYS = 7;
const NOTIFY_TOP = 10;
const CATEGORY_NAMES = { 4: 'ケージ4マスまで', 5: 'ケージ5マスまで', 6: 'ケージ6マスまで', 9: 'ケージ9マスまで' };

function previousMonth(month) {
  const [y, m] = month.split('-').map(Number);
  return new Date(Date.UTC(y, m - 2, 1)).toISOString().slice(0, 7);
}

function formatTime(sec) {
  const h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
  const ss = String(s).padStart(2, '0');
  return h ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`;
}

// 名前に含まれる Discord の書式記号を無効にする
const escapeMarkdown = (text) => text.replace(/([\\*_~`|>#\[\]()-])/g, '\\$1');

export async function notifyMonthly(env, now = Date.now()) {
  if (!env.DISCORD_WEBHOOK_URL) {
    console.log('DISCORD_WEBHOOK_URL が未設定のため通知しません');
    return;
  }
  const today = jstDate(now);
  if (Number(today.slice(8, 10)) > NOTIFY_DAYS) return;
  const month = previousMonth(today.slice(0, 7));
  const sent = await env.DB.prepare('SELECT 1 FROM notifications WHERE month = ?').bind(month).first();
  if (sent) return;

  await postMonthlyRanking(env, month);
  await env.DB.prepare('INSERT OR IGNORE INTO notifications (month, sent_at) VALUES (?, ?)')
    .bind(month, new Date(now).toISOString()).run();
  console.log(`${month} の月間ランキングを通知しました`);
}

// 通知のテスト送信(今月分、「テスト」と付けて送る。notifications 表には記録しない)。
// シークレット ADMIN_TOKEN を設定しているときだけ使える:
//   curl -X POST -H "authorization: Bearer <ADMIN_TOKEN>" https://<サイト>/api/admin/notify-test
async function notifyTest(request, env) {
  if (!env.ADMIN_TOKEN || request.headers.get('authorization') !== `Bearer ${env.ADMIN_TOKEN}`) {
    return json({ error: 'not found' }, 404);
  }
  if (!env.DISCORD_WEBHOOK_URL) return json({ error: 'DISCORD_WEBHOOK_URL が未設定です' }, 500);
  const month = jstDate().slice(0, 7);
  await postMonthlyRanking(env, month, true);
  return json({ ok: true, month });
}

export async function postMonthlyRanking(env, month, test = false) {
  const [y, m] = month.split('-').map(Number);
  const lines = [`## 🏆 ${test ? '【テスト】' : ''}${y}年${m}月のランキング`];
  let total = 0;
  for (const category of CATEGORIES) {
    const { results } = await env.DB.prepare(
      'SELECT name, seconds, date FROM records WHERE category = ? AND month = ? ORDER BY seconds, id LIMIT ?',
    ).bind(category, month, NOTIFY_TOP).all();
    const { n } = await env.DB.prepare('SELECT COUNT(*) AS n FROM records WHERE category = ? AND month = ?')
      .bind(category, month).first();
    total += n;
    if (!results.length) continue;
    lines.push('', `**${CATEGORY_NAMES[category]}**（${n}件）`);
    results.forEach((r, i) => {
      const medal = ['🥇', '🥈', '🥉'][i] || `${i + 1}.`;
      lines.push(`${medal} ${escapeMarkdown(r.name)}　${formatTime(r.seconds)}　(${r.date.slice(5).replace('-', '/')})`);
    });
  }
  if (!total) lines.push('', 'この月の記録はありませんでした。');
  if (env.SITE_URL) lines.push('', env.SITE_URL);

  const res = await fetch(env.DISCORD_WEBHOOK_URL, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    // 名前に @everyone などがあってもメンションしない
    body: JSON.stringify({ content: lines.join('\n').slice(0, 2000), allowed_mentions: { parse: [] } }),
  });
  if (!res.ok) throw new Error(`Discord への通知に失敗しました: ${res.status} ${await res.text()}`);
}

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
