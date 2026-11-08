# killer-sudoku-solver

キラーナンプレ（Killer Sudoku）のソルバーと問題生成器。

## ルール

- 9x9の盤面の各行・各列・各3x3ブロックに1〜9が1回ずつ入る（通常のナンプレと同じ）
- 盤面は「ケージ」（点線で囲まれたマスのグループ）に分割され、各ケージには合計値が指定されている
- ケージ内の数字の合計は指定値と一致し、ケージ内で同じ数字は重複しない

## 使い方

```sh
cargo build --release

# 問題生成(--seed で再現可能、--max-size でケージの最大マス数、--quiet でデータ行のみ)
./target/release/killer-sudoku-solver generate -n 3 --seed 1 --max-size 5

# 解く(1 行 1 問。ファイル省略時は標準入力)
./target/release/killer-sudoku-solver generate -n 10 --quiet > puzzles.txt
./target/release/killer-sudoku-solver solve puzzles.txt

# テスト
cargo test --release
```

### ブラウザで解く

```sh
# 20 問入りの play.html を書き出す(-n で問題数、--seed、--max-size も指定可)
./target/release/killer-sudoku-solver html -n 20 -o play.html

# 手持ちの問題ファイルから作る場合
./target/release/killer-sudoku-solver html --from puzzles.txt -o play.html
```

`play.html` は 1 ファイルで完結しているので、ダブルクリックで開けば遊べます(ネット接続不要)。
メモ、戻す/進む、重複の赤表示、答え合わせ、ケージの組み合わせ表示に対応し、途中経過はブラウザに自動保存されます。
「ヒント」ボタン(<kbd>H</kbd>)は、次に確定できるマスについて、1 回目で使う手筋と注目する場所を表示し、続けて押すと手順を 1 つずつ(消える候補を盤面に赤で示して)説明して、最後にマスと数字を表示します。
[web/player.html](web/player.html) を直接開いて、問題データを貼り付けて遊ぶこともできます。

### 公開用ページ(Cloudflare Pages)

生成器を WebAssembly にしてページに埋め込むと、ブラウザ上で「新しい問題」を生成できます
(生成した問題はそのブラウザに 30 問まで保存。URL の `#p=` で問題を共有できます)。

```sh
cargo rustc --release --lib --target wasm32-unknown-unknown --crate-type cdylib
cargo build --release
./target/release/killer-sudoku-solver html -n 0 \
  --wasm target/wasm32-unknown-unknown/release/killer_sudoku_solver.wasm -o dist/index.html

npx wrangler whoami   # デプロイ先のアカウントを確認してから
npx wrangler deploy   # wrangler.jsonc の設定で dist/ と worker/ を公開
```

#### タイムのランキング

公開サイトでは、生成した問題をヒント・答え合わせなしでクリアすると、名前を付けてタイムを登録できます
(「最初から」で解き直した問題は不可)。ランキングはケージの大きさ(4 / 5 / 6 / 9 マスまで)ごとに月間(日本時間)と通時。
名前はブラウザに覚えておき、最後に使った名前が初期値、登録済みの名前は選択できます。
API は [worker/index.js](worker/index.js)、データは Cloudflare D1。

```sh
# データベース(作成済み、ID は wrangler.jsonc)。migrations/ にテーブル変更を足したら:
npx wrangler d1 migrations apply DB --remote

# 手元で試す(ローカルの D1 を使う)
npx wrangler d1 migrations apply DB --local
npx wrangler dev
```

サーバーは解答が問題と合っているか、ヒント数字がないかなどを確かめますが、
タイム自体はブラウザの申告なので、改ざんを完全には防げません。

公開先: https://killer-sudoku.kyamotu-kyamotu.workers.dev/
(新しい Cloudflare Pages は Workers の静的アセットとして動くため、URL は workers.dev になる)

データ形式・設計・今後の予定は [PLAN.md](PLAN.md) を参照。
