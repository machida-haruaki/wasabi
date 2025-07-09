# WasabiOS 技術解説ドキュメントシステム

このディレクトリには、WasabiOSの実装に関する技術解説ページが含まれています。

## 📁 ディレクトリ構成

```
docs/
├── index.html              # メインインデックスページ
├── template.html           # 技術解説ページのテンプレート
├── README.md              # このファイル
├── assets/                # 共通リソース
│   ├── style.css          # 共通スタイルシート
│   └── script.js          # 共通JavaScript
└── topics/                # 各技術トピック
    ├── font-cache/        # フォントキャッシュシステム
    │   └── index.html
    ├── memory-management/ # メモリ管理（予定）
    ├── graphics-system/   # グラフィックシステム（予定）
    └── ...
```

## 🚀 新しい技術解説ページの作成方法

### 基本的な指示フォーマット

```
技術解説ページを作成してください：

**基本情報:**
- トピック: [技術名]
- カテゴリ: [グラフィックシステム/メモリ管理/システム基盤/開発ツール]
- 難易度: [初級/中級/上級]
- 関連ファイル: [src/xxx.rs, src/yyy.rs]

**解説内容:**
- [具体的に解説したい内容を記述]
- [実装のポイントや難しい部分]
- [パフォーマンスの改善点など]

**特別な要求:**
- [図示したい内容]
- [アニメーションで表現したい部分]
- [インタラクティブな要素]
```

### 具体的な例

```
技術解説ページを作成してください：

**基本情報:**
- トピック: メモリアロケータ
- カテゴリ: メモリ管理
- 難易度: 中級
- 関連ファイル: src/allocator.rs

**解説内容:**
- ヒープメモリの管理方法
- アロケーションアルゴリズムの比較
- フラグメンテーションの問題と対策
- Rustの所有権システムとの関係

**特別な要求:**
- メモリレイアウトの図示
- アロケーション/デアロケーションのアニメーション
- 異なるアルゴリズムのパフォーマンス比較チャート
```

## 🎨 利用可能なコンポーネント

### 1. ステップバイステップ解説
```html
<div id="step0" class="step">
    <h2>📋 ステップタイトル</h2>
    <p>解説内容...</p>
</div>
```

### 2. 比較レイアウト
```html
<div class="comparison">
    <div class="before">
        <h3>🐌 変更前</h3>
        <p>内容...</p>
    </div>
    <div class="after">
        <h3>⚡変更後</h3>
        <p>内容...</p>
    </div>
</div>
```

### 3. コードブロック
```html
<div class="code-block">
<span class="rust-keyword">fn</span> example() {
    <span class="rust-comment">// コメント</span>
    <span class="rust-keyword">let</span> x = <span class="rust-string">"hello"</span>;
}
</div>
```

### 4. メモリ図
```html
<div class="memory-diagram">
    <h4>メモリレイアウト</h4>
    <div class="array-3d" id="memoryExample">
        <!-- JavaScriptで動的生成 -->
    </div>
</div>
```

### 5. パフォーマンスチャート
```html
<div class="performance-chart" id="perfChart">
    <!-- JavaScriptで動的生成 -->
</div>
```

## 🛠️ JavaScript ユーティリティ

### TechUtils.generate3DArray()
3次元配列の視覚化
```javascript
TechUtils.generate3DArray('containerId', fontData, 16, 8);
```

### TechUtils.generatePerformanceChart()
パフォーマンス比較チャート
```javascript
const data = [
    { label: '1回目', complexity: 'O(n)', height: 80, fast: false },
    { label: '2回目', complexity: 'O(1)', height: 20, fast: true }
];
TechUtils.generatePerformanceChart('chartId', data);
```

### TechUtils.highlightCode()
Rustコードのシンタックスハイライト
```javascript
TechUtils.highlightCode(document.getElementById('codeBlock'));
```

## 📝 テンプレート変数

テンプレートファイル（`template.html`）で使用可能な変数：

- `{{TITLE}}`: ページタイトル
- `{{SUBTITLE}}`: サブタイトル
- `{{ICON}}`: アイコン絵文字
- `{{CATEGORY}}`: カテゴリID
- `{{CATEGORY_NAME}}`: カテゴリ名
- `{{CONTENT}}`: メインコンテンツ
- `{{RELATED_FILES}}`: 関連ファイル
- `{{STEP_SPECIFIC_JS}}`: ステップ固有のJavaScript
- `{{CUSTOM_JS}}`: カスタムJavaScript

## 🎯 品質ガイドライン

### 内容の品質
- **初心者にも理解できる**: 専門用語には説明を付ける
- **段階的な説明**: 複雑な概念は小さなステップに分割
- **視覚的な表現**: 図やアニメーションを積極的に活用
- **実践的な例**: 実際のコードと関連付ける

### 技術的な品質
- **レスポンシブデザイン**: モバイルでも見やすく
- **アクセシビリティ**: キーボード操作に対応
- **パフォーマンス**: 軽量で高速な読み込み
- **一貫性**: 統一されたデザインとナビゲーション

## 🔄 更新プロセス

1. **新しいトピックの追加**
   - `docs/topics/[topic-name]/` フォルダを作成
   - `index.html` を作成（テンプレートを使用）
   - `docs/index.html` のトピックリストを更新

2. **既存トピックの更新**
   - 該当する `index.html` を編集
   - 必要に応じて統計情報を更新

3. **共通機能の改善**
   - `assets/style.css` または `assets/script.js` を更新
   - 全ページに影響するため慎重に検証

## 📊 統計情報の更新

`docs/index.html` の統計は手動で更新する必要があります：

```javascript
document.getElementById('completedCount').textContent = [完了数];
document.getElementById('inProgressCount').textContent = [作業中数];
document.getElementById('plannedCount').textContent = [予定数];
```

## 🎨 カスタマイズ

### 新しいカテゴリの追加
1. `docs/index.html` に新しいカテゴリセクションを追加
2. 適切なアイコンと色を選択
3. カテゴリIDを設定

### 新しいコンポーネントの追加
1. `assets/style.css` にスタイルを追加
2. `assets/script.js` にJavaScript機能を追加
3. `template.html` を必要に応じて更新

---

このシステムを使用して、WasabiOSの実装過程を分かりやすく文書化し、学習リソースとして活用してください。
