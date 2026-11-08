-- クリアタイムの記録。1 行 = 1 回のクリア。
CREATE TABLE records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  seconds INTEGER NOT NULL,
  category INTEGER NOT NULL,      -- ケージの最大マス数(生成時の設定: 4 / 5 / 6 / 9)
  puzzle TEXT NOT NULL,           -- 「ケージラベル;合計値」(同じ問題・同じ名前の二重登録を防ぐ)
  month TEXT NOT NULL,            -- 日本時間の YYYY-MM(月間ランキング用)
  date TEXT NOT NULL,             -- 日本時間の YYYY-MM-DD
  created_at TEXT NOT NULL,       -- UTC の ISO 8601
  UNIQUE (puzzle, name)
);
CREATE INDEX records_all ON records (category, seconds, id);
CREATE INDEX records_month ON records (category, month, seconds, id);
