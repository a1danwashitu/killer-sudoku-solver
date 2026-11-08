-- Discord に月間ランキングを通知した月(二重送信を防ぐ)
CREATE TABLE notifications (
  month TEXT PRIMARY KEY,         -- 通知した月(日本時間の YYYY-MM)
  sent_at TEXT NOT NULL           -- UTC の ISO 8601
);
