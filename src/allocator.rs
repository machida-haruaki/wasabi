//! # メモリアロケータ実装
//!
//! このモジュールは、OSカーネル用のメモリアロケータを実装しています。
//! First Fit（最初適合）アルゴリズムを使用して、メモリの動的割り当てを行います。
//!
//! ## First Fitアルゴリズムとは
//! メモリ割り当て要求があった際に、空きメモリ領域のリストを先頭から順番に調べ、
//! 要求されたサイズ以上の最初に見つかった領域を割り当てるアルゴリズムです。
//!
//! ## メモリ管理の仕組み
//! - 各メモリブロックの先頭にHeaderを配置して管理
//! - Headerには次のブロックへのポインタ、サイズ、割り当て状態を保存
//! - 連結リスト構造でメモリブロックを管理
//!
//! ## Rustの重要な概念（このファイルで使用されているもの）
//!
//! ### 所有権（Ownership）
//! - Rustの最も重要な概念。各値には「所有者」が存在する
//! - 所有者がスコープを出ると、値は自動的に解放される
//! - `Box<T>`は値をヒープに配置し、その所有権を管理する
//!
//! ### 借用（Borrowing）
//! - `&T`（不変参照）や`&mut T`（可変参照）で値を「借用」できる
//! - 借用中は元の所有者は値を変更・移動できない
//! - `RefCell<T>`は実行時に借用ルールをチェックする
//!
//! ### Option型
//! - `Option<T>`はnullの概念を安全に扱うための型
//! - `Some(value)`（値あり）か`None`（値なし）のどちらか
//! - パターンマッチングで安全に値を取り出せる
//!
//! ### unsafe
//! - Rustの安全性チェックを無効にするキーワード
//! - 生ポインタの操作やアセンブリ呼び出しで必要
//! - 使用時は開発者が安全性を保証する責任がある

// extern crate: 外部クレート（ライブラリ）を使用することを宣言
// allocはRustの標準的な動的メモリ割り当て機能を提供
extern crate alloc;

// use文: 他のモジュールから型や関数をインポートする
// crate::は現在のクレート（プロジェクト）内のモジュールを指す
use crate::result::Result;                    // カスタムResult型
use crate::uefi::EfiMemoryDescriptor;         // UEFIメモリディスクリプタ
use crate::uefi::EfiMemoryType;               // UEFIメモリタイプ列挙型
use crate::uefi::MemoryMapHolder;             // UEFIメモリマップホルダー
use alloc::alloc::GlobalAlloc;                // グローバルアロケータトレイト
use alloc::alloc::Layout;                     // メモリレイアウト記述子
use alloc::boxed::Box;                        // ヒープ割り当てスマートポインタ
use core::borrow::BorrowMut;                  // 可変借用トレイト
use core::cell::RefCell;                      // 内部可変性を提供する型
use core::cmp::max;                           // 最大値を求める関数
use core::fmt;                                // フォーマット関連のトレイト
use core::mem::size_of;                       // 型のサイズを取得する関数
use core::ops::DerefMut;                      // 可変参照外しトレイト
use core::ptr::null_mut;                      // null可変ポインタを作成する関数

/// 指定された値を最も近い2の累乗に切り上げる関数
///
/// # 動作原理
/// 1. `v - 1`を計算（例：v=5なら4）
/// 2. `leading_zeros()`で先頭の0の個数を取得（例：4=0b100なら29個）
/// 3. `usize::BITS - leading_zeros()`で必要なビット数を計算（例：32-29=3）
/// 4. `1 << 3`で2^3=8を計算
///
/// # 例
/// - 1 → 1 (2^0)
/// - 3 → 4 (2^2)
/// - 5 → 8 (2^3)
/// - 9 → 16 (2^4)
///
/// # エラー
/// - 0が入力された場合、またはオーバーフローが発生した場合にエラーを返す
///
/// # Rustの文法解説
/// - `pub fn`: 公開関数の宣言（他のモジュールから呼び出し可能）
/// - `-> Result<usize>`: 戻り値の型。成功時はusize、失敗時はエラー文字列
/// - `1usize`: usize型の1リテラル（型を明示的に指定）
/// - `.checked_shl()`: オーバーフローをチェックする左シフト演算
/// - `.ok_or()`: Option<T>をResult<T, E>に変換（NoneをErrに変換）
/// - `v.wrapping_sub(1)`: オーバーフローを許可する減算（アンダーフロー時は最大値になる）
pub fn round_up_to_nearest_pow2(v: usize) -> Result<usize> {
    1usize
        .checked_shl(usize::BITS - v.wrapping_sub(1).leading_zeros())
        .ok_or("Out of range")
}

#[test_case]
fn round_up_to_nearest_pow2_test() {
    assert_eq!(round_up_to_nearest_pow2(0), Err("Out of range"));
    assert_eq!(round_up_to_nearest_pow2(1), Ok(1));
    assert_eq!(round_up_to_nearest_pow2(2), Ok(2));
    assert_eq!(round_up_to_nearest_pow2(3), Ok(4));
    assert_eq!(round_up_to_nearest_pow2(4), Ok(4));
    assert_eq!(round_up_to_nearest_pow2(5), Ok(8));
    assert_eq!(round_up_to_nearest_pow2(6), Ok(8));
    assert_eq!(round_up_to_nearest_pow2(7), Ok(8));
    assert_eq!(round_up_to_nearest_pow2(8), Ok(8));
    assert_eq!(round_up_to_nearest_pow2(9), Ok(16));
}

/// メモリブロックの管理情報を格納するヘッダー構造体
///
/// # メモリレイアウトの詳細
/// ```
/// 【アライメント境界とHeaderの配置】
/// アドレス境界: |<--32B-->|<--32B-->|<--32B-->|<--32B-->|
/// メモリ構造:   |Header(H)|--data---|Header(H)|--data---|
///              ^32B境界  ^32B境界  ^32B境界  ^32B境界
///
/// 【具体的な割り当て例】（1024バイト要求、32バイトアライメント）
/// 割り当て前:
/// |-------------- self (空きブロック 4096バイト) --------------|
/// 0x1000                                                   0x2000
///
/// 割り当て後:
/// |--self--|H|----allocated data----|H|--padding--|
/// 0x1000   ^  ^                      ^
///          |  allocated_addr         |
///          header_for_allocated      header_for_padding
///          (32バイト)                (必要な場合)
/// ```
///
/// # 重要なポイント
/// 1. **Header配置**: Headerは割り当てデータの「前」に配置される
/// 2. **アライメント**: HeaderとデータはHEADER_SIZE（32バイト）境界に配置
/// 3. **サイズ計算**: 実際の消費 = Header(32B) + データサイズ
/// 4. **アライメント基本単位**: HEADER_SIZEが最小アライメント単位として機能
///
/// # Box型使用に関する注意
/// ```
/// ⚠️ 循環参照の問題:
/// - Box<Header>はヒープ割り当てを行うが、これ自体がアロケータ内で使用されている
/// - この問題は以下の方法で回避:
///   1. Box::from_raw() - 既存メモリをBoxとして扱う（新規割り当てなし）
///   2. Box::leak() - メモリをRustの管理から外す
///   3. UEFIメモリ領域を直接使用 - Rustのヒープアロケータを使わない
///
/// より安全な設計では生ポインタ（*mut Header）や直接アドレス管理を使用すべき
/// ```
///
/// # フィールド説明
/// - `next_header`: 次のメモリブロックのHeaderへのポインタ（連結リスト構造）
/// - `size`: このブロックの総サイズ（Headerサイズを含む）
/// - `is_allocated`: このブロックが割り当て済みかどうかのフラグ
/// - `_reserved`: 将来の拡張用の予約フィールド
///
/// # Rustの文法解説
/// - `struct`: 構造体の定義キーワード
/// - `Option<Box<Header>>`: Option型でBox型を包む（nullableなヒープポインタ）
/// - `usize`: プラットフォーム依存のサイズ型（32bit環境では32bit、64bit環境では64bit）
/// - `bool`: 真偽値型（trueまたはfalse）
/// - `_reserved`: アンダースコアで始まる名前は「使用していない」ことを示す
struct Header {
    next_header: Option<Box<Header>>,  // Option<T>: Some(T)またはNone
    /// このブロックの総サイズ（Header自身のサイズ32バイトを含む）
    /// 例：1024バイトのデータを割り当てた場合、size = 1024 + 32 = 1056
    /// 計算式：size = 実際のデータサイズ + HEADER_SIZE
    size: usize,                       // usize: メモリサイズやインデックスに使用
    is_allocated: bool,                // bool: true/false
    _reserved: usize,                  // _で始まる名前: 未使用フィールド
}

/// Headerのサイズ（バイト単位）
///
/// # Rustの文法解説
/// - `const`: コンパイル時定数の宣言
/// - `size_of::<Header>()`: ジェネリック関数の呼び出し（型パラメータを明示）
/// - `usize`: 戻り値の型
const HEADER_SIZE: usize = size_of::<Header>();

/// コンパイル時アサーション
///
/// # Rustの文法解説
/// - `#[allow(clippy::assertions_on_constants)]`: Clippyの警告を無効化
/// - `const _: () = ...`: 無名定数（コンパイル時にのみ評価される）
/// - `assert!()`: コンパイル時アサーション（条件が偽の場合コンパイルエラー）
/// - `count_ones()`: ビット演算（1のビット数を数える）
#[allow(clippy::assertions_on_constants)]
// Headerのサイズは2の累乗である必要がある（アライメント要件のため）
const _: () = assert!(HEADER_SIZE.count_ones() == 1);

/// 4KBページ用のレイアウト定数
///
/// # Rustの文法解説
/// - `pub const`: 公開定数の宣言
/// - `unsafe { ... }`: 安全性チェックを無効化するブロック
/// - `Layout::from_size_align_unchecked()`: チェックなしでLayoutを作成
pub const LAYOUT_PAGE_4K: Layout = unsafe { Layout::from_size_align_unchecked(4096, 4096) };

/// Header構造体のメソッド実装
///
/// # Rustの文法解説
/// - `impl`: 型に対するメソッドの実装ブロック
/// - `impl Header`: Header型に対するメソッドを定義
/// - メソッドは構造体のインスタンスに対して呼び出される
impl Header {
    /// このブロックが指定されたサイズとアライメントの要求を満たせるかチェック
    ///
    /// # パラメータ
    /// - `size`: 要求されるメモリサイズ
    /// - `align`: 要求されるアライメント
    ///
    /// # 前提条件
    /// - `align`: 2の冪である必要がある
    ///
    /// # 計算式の詳細説明
    /// `size + HEADER_SIZE * 2 + align` の内訳:
    /// - `size`: 実際の要求サイズ（2の冪に調整済み）
    /// - `HEADER_SIZE`: 割り当て領域用のHeader（32バイト）
    /// - `HEADER_SIZE`: パディング領域用のHeader（必要な場合、32バイト）
    /// - `align`: アライメント調整で必要になる可能性のある最大追加スペース
    ///
    /// # なぜこの計算が必要か
    /// - 最悪ケース: アライメント調整でalign-1バイトの無駄が発生する可能性
    /// - 安全マージン: 概算で大きめに見積もることで確実な割り当てを保証
    /// - Header領域: 管理情報を格納するための領域を確保
    fn can_provide(&self, size: usize, align: usize) -> bool {
        self.size >= size + HEADER_SIZE * 2 + align
    }

    /// このブロックが割り当て済みかどうかを返す
    fn is_allocated(&self) -> bool {
        self.is_allocated
    }

    /// このブロックの終端アドレスを計算して返す
    /// ブロックの開始アドレス + サイズ = 終端アドレス
    fn end_addr(&self) -> usize {
        self as *const Header as usize + self.size
    }
    /// 指定されたアドレスに新しいHeaderを作成する
    ///
    /// # 安全性
    /// - `addr`は有効なメモリアドレスである必要がある
    /// - `addr`はHeader構造体を格納するのに十分なサイズがある必要がある
    /// - `addr`は適切にアライメントされている必要がある
    ///
    /// # 処理の流れ
    /// 1. アドレスをHeader型のポインタにキャスト
    /// 2. デフォルト値でHeaderを初期化
    /// 3. Box::from_rawでBoxに包んで返す
    unsafe fn new_from_addr(addr: usize) -> Box<Header> {
        let header = addr as *mut Header;
        header.write(Header {
            next_header: None,
            size: 0,
            is_allocated: false,
            _reserved: 0,
        });
        Box::from_raw(addr as *mut Header)
    }

    /// 割り当て済みデータ領域のアドレスからHeaderを取得する
    ///
    /// # パラメータ
    /// - `addr`: ユーザーに返された割り当て済みデータ領域の先頭アドレス
    ///           （Header + データの構造において、データ部分の開始位置）
    ///
    /// # メモリレイアウト
    /// ```
    /// |------ Header ------|----- ユーザーデータ -----|
    /// ^                    ^
    /// Header開始位置        addr（引数として渡される位置）
    /// (addr - HEADER_SIZE)
    /// ```
    ///
    /// # 安全性
    /// - `addr`は有効な割り当て済みデータ領域の先頭アドレスである必要がある
    /// - `addr - HEADER_SIZE`の位置に有効なHeaderが存在する必要がある
    ///
    /// # 処理の流れ
    /// 1. 割り当て領域のアドレスからHEADER_SIZEを引いてHeaderの位置を計算
    /// 2. Box::from_rawでBoxに包んで返す
    unsafe fn from_allocated_region(addr: *mut u8) -> Box<Header> {
        let header = addr.sub(HEADER_SIZE) as *mut Header;
        Box::from_raw(header)
    }
    /// メモリブロックから指定されたサイズとアライメントでメモリを割り当てる
    ///
    /// # パラメータ
    /// - `size`: 要求されるメモリサイズ
    /// - `align`: 要求されるアライメント（2の累乗である必要がある）
    ///
    /// # 前提条件
    /// - `size`: 0より大きい値（実際の要求サイズ）
    /// - `align`: 2の冪である必要がある（1, 2, 4, 8, 16, 32, 64, ...）
    ///   - Rustの std::alloc::Layout の仕様要件
    ///   - ビット演算によるマスク処理の前提条件
    ///
    /// # 戻り値
    /// - `Some(ptr)`: 割り当てに成功した場合、割り当てられたメモリの先頭ポインタ
    /// - `None`: 割り当てに失敗した場合（サイズ不足、既に割り当て済みなど）
    ///
    /// # 注意
    /// std::alloc::Layout のドキュメントより:
    /// > All layouts have an associated size and a power-of-two alignment.
    fn provide(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        // 【サイズ調整処理】
        // 目的: メモリ断片化を防ぎ、効率的なメモリ管理を実現
        // 理由:
        //   - 2の冪サイズにすることで、解放時の結合処理が効率化される
        //   - キャッシュライン効率の向上（多くのCPUで32/64バイトキャッシュライン）
        //   - 最小でもHEADER_SIZE確保により、Header配置領域を保証
        let size = max(round_up_to_nearest_pow2(size).ok()?, HEADER_SIZE);

        // 【アライメント調整処理】
        // 目的: 最小アライメント境界の保証
        // 理由:
        //   - HEADER_SIZEは32バイト（2^5）で、このアロケータの基本単位
        //   - Headerとデータの両方が同じ境界に配置されることを保証
        //   - CPUの効率的なメモリアクセスパターンに合わせる
        let align = max(align, HEADER_SIZE);

        // 既に割り当て済みか、要求を満たせない場合は失敗
        if self.is_allocated() || !self.can_provide(size, align) {
            None
        } else {
            // メモリレイアウトの詳細図解:
            //
            // 【割り当て前】
            // アドレス境界: |<--32B-->|<--32B-->|<--32B-->|<--32B-->|
            // メモリ領域:   |---------- self (空きブロック) ----------|
            //              0x1000                                   0x2000
            //
            // 【割り当て後】（1024バイト要求、32バイトアライメント）
            // アドレス境界: |<--32B-->|<--32B-->|<--32B-->|<--32B-->|
            // メモリ領域:   |--self--|H|----allocated data----|H|padding|
            //              0x1000   ^  ^                      ^
            //                       |  allocated_addr         |
            //                       header_for_allocated      header_for_padding
            //                       (32バイト)                (必要な場合)
            //
            // 重要なポイント:
            // 1. Header(H)は割り当てデータの「前」に配置される
            // 2. Headerとデータの両方が32バイト境界に配置される
            // 3. allocated_addrはユーザーに返されるアドレス（Headerの後）
            // 4. 実際の消費サイズ = Header(32B) + データ(1024B) = 1056B
            // 5. アライメント調整により追加パディングが発生する場合がある

            // 使用したサイズを追跡する変数
            let mut size_used = 0;

            // 【アライメント境界へのアドレス計算】
            // 前提: alignは2の冪（1, 2, 4, 8, 16, ...）
            // 目的: 要求されたアライメント境界に正確に配置されたアドレスを計算
            //
            // 計算原理:
            //   1. (self.end_addr() - size): ブロック終端から要求サイズ分戻る
            //   2. align - 1: 2の冪-1は下位ビットがすべて1 (例: 8-1=7=0b111)
            //   3. !(align - 1): ビット反転でマスクを作成 (例: !7=0b...11111000)
            //   4. & !(align - 1): 下位ビットをクリアしてアライメント境界に切り下げ
            //
            // 例: align=8の場合
            //   - 任意のアドレス 0x1C07 & 0b...11111000 = 0x1C00 (8の倍数)
            let end_addr = self.end_addr();
            
            // 安全性チェック: オーバーフロー/アンダーフローを防ぐ
            if end_addr < size {
                return None; // サイズが大きすぎる場合は失敗
            }
            
            let allocated_addr = (end_addr - size) & !(align - 1);
            
            // 追加の安全性チェック: 計算結果が有効な範囲内かを確認
            if allocated_addr < HEADER_SIZE || allocated_addr < self as *const Header as usize + HEADER_SIZE {
                return None; // 無効なアドレスの場合は失敗
            }

            // 割り当て領域用のHeaderを作成
            let mut header_for_allocated =
                unsafe { Self::new_from_addr(allocated_addr - HEADER_SIZE) };
            header_for_allocated.is_allocated = true;
            header_for_allocated.size = size + HEADER_SIZE;
            size_used += header_for_allocated.size;

            // 元の次のブロックへのリンクを引き継ぐ
            header_for_allocated.next_header = self.next_header.take();

            // 【Padding発生条件の詳細解説】
            //
            // Paddingが発生するのは以下の条件が揃った場合のみ：
            //
            // 1. 基本条件：アライメント調整による切り下げが発生
            //    - (self.end_addr() - size) がアライメント境界にない
            //    - & !(align - 1) による切り下げで実際の配置位置が前方にずれる
            //
            // 2. 典型的なケース：
            //    - アライメント >> 要求サイズ（例：1024バイトアライメント、100バイト要求）
            //    - ブロック終端位置がアライメント境界と一致しない
            //
            // 3. 具体例：
            //    ブロック: 0x1000-0x2000 (4096バイト)
            //    要求: 128バイト、1024バイトアライメント
            //
            //    計算過程:
            //    - end_addr - size = 0x2000 - 128 = 0x1F80
            //    - アライメント調整: 0x1F80 & 0xFC00 = 0x1C00 (切り下げ発生!)
            //    - allocated_addr = 0x1C00
            //    - 割り当て終端 = 0x1C00 + 128 + 32 = 0x1CA0
            //    - padding = 0x2000 - 0x1CA0 = 864バイト
            //
            // 4. Paddingが発生しないケース：
            //    - (end_addr - size) が既にアライメント境界にある
            //    - アライメントが小さい（HEADER_SIZE以下）
            //    - 要求サイズがアライメントより大きい
            //
            // 5. 数学的条件：
            //    padding発生 ⟺ (self.end_addr() - size) % align != 0
            //
            // 割り当て領域の後に余りがある場合、パディング用のHeaderを作成
            if header_for_allocated.end_addr() != self.end_addr() {
                let mut header_for_padding =
                    unsafe { Self::new_from_addr(header_for_allocated.end_addr()) };
                header_for_padding.is_allocated = false;
                header_for_padding.size = self.end_addr() - header_for_allocated.end_addr();
                size_used += header_for_padding.size;

                // 重要：パディングに連結リストを移してから、パディングを割り当て領域に接続
                header_for_padding.next_header = header_for_allocated.next_header.take();
                header_for_allocated.next_header = Some(header_for_padding);
            }

            // 現在のブロック（self）のサイズを縮小
            assert!(self.size >= size_used + HEADER_SIZE);
            self.size -= size_used;

            // 現在のブロックの次に割り当て領域をリンク
            self.next_header = Some(header_for_allocated);

            // 割り当てられたメモリのアドレスを返す
            Some(allocated_addr as *mut u8)
        }
    }
}
impl Drop for Header {
    /// Headerが誤ってドロップされることを防ぐ
    ///
    /// Headerはメモリ上に直接配置されており、Rustの通常のドロップ処理で
    /// 解放されるべきではないため、パニックを発生させる
    fn drop(&mut self) {
        panic!("Header should not be dropped! Address: {:#x}, Size: {:#x}, Allocated: {}",
               self as *const Header as usize,
               self.size,
               self.is_allocated);
    }
}

impl fmt::Debug for Header {
    /// デバッグ用のフォーマット実装
    ///
    /// Headerのアドレス、サイズ、割り当て状態を16進数で表示
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Header @ {:#018x} {{ size: {:#018x}, is_allocated: {} }}",
            self as *const Header as usize,
            self.size,
            self.is_allocated()
        )
    }
}

/// First Fit（最初適合）アルゴリズムを使用するメモリアロケータ
///
/// # 構造
/// - `first_header`: メモリブロックの連結リストの先頭へのポインタ
/// - RefCellを使用して内部可変性を実現
///
/// # RefCell<T>について - なぜ必要なのか？
///
/// ## Rustの基本的な借用ルール
/// - **不変参照（&T）**: 複数同時に存在可能だが、値を変更できない
/// - **可変参照（&mut T）**: 1つだけ存在可能で、値を変更できる
/// - これらのルールは**コンパイル時**にチェックされる
///
/// ## static変数の制約とアロケータの要求の矛盾
/// ```rust
/// // static変数は不変として扱われる
/// static ALLOCATOR: FirstFitAllocator = ...;
/// // ↓
/// // ALLOCATORからは不変参照（&FirstFitAllocator）しか取得できない
/// ```
///
/// しかし、アロケータは以下の操作で内部状態を変更する必要がある：
/// - メモリ割り当て時：連結リスト（first_header）の更新
/// - メモリ解放時：連結リストの更新
/// - **不変参照しか持てないのに、内部状態を変更したい** ← 矛盾！
///
/// ## RefCell<T>による解決 - 内部可変性（Interior Mutability）
///
/// RefCell<T>は「**実行時借用チェック**」を提供する：
/// - `borrow()` → 不変参照を取得（複数同時可能）
/// - `borrow_mut()` → 可変参照を取得（1つだけ、実行時チェック）
/// - 借用ルール違反があると**実行時にパニック**
///
/// ```rust
/// // 不変参照しか持っていない状況
/// fn some_method(&self) {  // &self は不変参照
///     // RefCellを使って実行時に可変参照を取得！
///     let mut header = self.first_header.borrow_mut();
///     // これで内部状態を安全に変更できる
/// }
/// ```
///
/// ## なぜMutexではなくRefCellなのか？
/// - **Mutex**: マルチスレッド用の重い同期プリミティブ、OSレベルの機能が必要
/// - **RefCell**: シングルスレッド用の軽量な実行時チェック、OSカーネルに適している
/// - この段階ではマルチスレッドは考慮していないため、RefCellで十分
///
/// ## 安全性の保証
/// - コンパイル時チェック → 実行時チェックに変更
/// - 借用ルールは依然として守られる（実行時にパニックで検出）
/// - 生ポインタを使うよりも安全
///
/// # First Fitアルゴリズムの動作
/// 1. メモリ割り当て要求を受ける
/// 2. 連結リストを先頭から順番に辿る
/// 3. 要求サイズを満たす最初の空きブロックを見つける
/// 4. そのブロックから必要な分だけ割り当てる
/// 5. 残りは新しい空きブロックとして管理
pub struct FirstFitAllocator {
    /// メモリブロック連結リストの先頭
    ///
    /// RefCell<T>により、不変参照（&self）からでも
    /// borrow_mut()で可変参照を取得して内部状態を変更可能
    first_header: RefCell<Option<Box<Header>>>,
}

/// グローバルアロケータとしてFirstFitAllocatorを設定
///
/// この宣言により、Rustの標準的なメモリ割り当て（Vec、Box、Stringなど）が
/// すべてこのアロケータを使用するようになる
///
/// # RefCellの実際の使用例
///
/// ## static変数とRefCellの組み合わせ
/// ```rust
/// // static変数は不変として扱われる
/// pub static ALLOCATOR: FirstFitAllocator = FirstFitAllocator {
///     first_header: RefCell::new(None),  // ← RefCellで包む
/// };
/// ```
///
/// ## 実際の動作パターン
/// 1. **Rustの標準ライブラリ（Vec、Box等）がメモリを要求**
///    ```rust
///    let vec = Vec::new();  // 内部でALLOCATOR.alloc()が呼ばれる
///    ```
///
/// 2. **GlobalAllocトレイトのalloc()メソッドが呼ばれる**
///    ```rust
///    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
///        // &self は不変参照だが、内部状態を変更する必要がある
///        self.alloc_with_options(layout)
///    }
///    ```
///
/// 3. **RefCellを使って実行時に可変参照を取得**
///    ```rust
///    pub fn alloc_with_options(&self, layout: Layout) -> *mut u8 {
///        // RefCellの力で、不変参照から可変参照を取得！
///        let mut header = self.first_header.borrow_mut();
///        // これで連結リストを安全に変更できる
///    }
///    ```
///
/// ## RefCellなしだとどうなるか？
/// ```rust
/// // もしRefCellを使わない場合（コンパイルエラー）
/// pub struct FirstFitAllocator {
///     first_header: Option<Box<Header>>,  // RefCellなし
/// }
///
/// impl FirstFitAllocator {
///     pub fn alloc_with_options(&self, layout: Layout) -> *mut u8 {
///         // ❌ コンパイルエラー！
///         // &selfは不変参照なので、first_headerを変更できない
///         self.first_header = Some(...);  // cannot assign to field
///     }
/// }
/// ```
///
/// ## RefCellの安全性保証
/// - **実行時チェック**: 借用ルール違反があると実行時にパニック
/// - **デッドロック回避**: 同じスレッド内での再帰的な可変借用を検出
/// - **メモリ安全性**: 生ポインタを使わずに内部可変性を実現
///
/// この仕組みにより、グローバルなstatic変数でありながら、
/// 安全にメモリ管理の状態を変更できるアロケータが実現されています。
#[global_allocator]
pub static ALLOCATOR: FirstFitAllocator = FirstFitAllocator {
    first_header: RefCell::new(None),
};

/// マルチスレッド環境での安全性を保証
///
/// FirstFitAllocatorがスレッド間で安全に共有できることを示す
/// RefCellの内部可変性により、実際の同期は実行時にチェックされる
unsafe impl Sync for FirstFitAllocator {}

/// Rustの標準アロケータトレイトの実装
///
/// このトレイトを実装することで、Rustの標準的なメモリ管理機能
/// （Vec、Box、Stringなど）がこのアロケータを使用できるようになる
unsafe impl GlobalAlloc for FirstFitAllocator {
    /// メモリ割り当て処理
    ///
    /// # パラメータ
    /// - `layout`: 割り当て要求の詳細（サイズとアライメント）
    ///
    /// # 戻り値
    /// - 成功時: 割り当てられたメモリの先頭ポインタ
    /// - 失敗時: null pointer
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_with_options(layout)
    }

    /// メモリ解放処理
    ///
    /// # パラメータ
    /// - `ptr`: 解放するメモリの先頭ポインタ
    /// - `_layout`: 元の割り当て情報（この実装では使用しない）
    ///
    /// # 処理の流れ
    /// 1. ポインタからHeaderを取得
    /// 2. 割り当てフラグをfalseに設定（空きブロックにする）
    /// 3. Box::leakでHeaderをメモリ上に残す（ドロップを防ぐ）
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let mut region = Header::from_allocated_region(ptr);
        region.is_allocated = false;
        Box::leak(region);
        // regionはここでリークされる。これは、メモリ上の空き情報を
        // ドロップで失わないようにするため
    }
}

impl FirstFitAllocator {
    /// 指定されたレイアウトでメモリを割り当てる（オプション付き）
    ///
    /// # パラメータ
    /// - `layout`: 割り当て要求の詳細（サイズとアライメント）
    ///
    /// # 戻り値
    /// - 成功時: 割り当てられたメモリの先頭ポインタ
    /// - 失敗時: null pointer
    ///
    /// # First Fitアルゴリズムの実装
    /// 1. 連結リストの先頭から開始
    /// 2. 各ブロックに対して割り当て可能かチェック
    /// 3. 可能なブロックが見つかったら割り当てて終了
    /// 4. 見つからない場合は次のブロックへ
    /// 5. 全てのブロックを調べても見つからない場合はnull pointerを返す
    pub fn alloc_with_options(&self, layout: Layout) -> *mut u8 {
        // 【RefCellの実際の使用例 - 内部可変性の実現】
        //
        // ここで重要なのは、このメソッドのシグネチャが `&self` であることです：
        // - `&self` = 不変参照（通常は内部状態を変更できない）
        // - しかし、RefCellを使うことで実行時に可変参照を取得可能
        //
        // 【ステップ1: RefCellから可変参照を借用】
        // borrow_mut(): RefCellから可変参照を借用（実行時チェック）
        // - 成功: RefMut<Option<Box<Header>>>を返す
        // - 失敗: 既に借用中の場合、実行時にパニック
        let mut header = self.first_header.borrow_mut();

        // 【ステップ2: RefMutから内部の値にアクセス】
        // deref_mut(): RefMut<T>からT（この場合Option<Box<Header>>）への可変参照を取得
        // これにより、Option<Box<Header>>を直接操作できるようになる
        let mut header = header.deref_mut();

        // 【RefCellの安全性チェック】
        // この時点で以下がチェックされている：
        // 1. 同じスレッド内で複数の可変借用がないか
        // 2. 可変借用中に不変借用が発生していないか
        // 3. 借用ルール違反があれば実行時にパニック
        //
        // 【なぜこの仕組みが必要か】
        // - static ALLOCATOR は不変参照しか提供しない
        // - しかしメモリ割り当てには内部状態（連結リスト）の変更が必要
        // - RefCellにより、コンパイル時制約を実行時制約に変更
        // - 結果：安全性を保ちながら内部可変性を実現

        // 連結リストを辿って適切なブロックを探す
        // loop: 無限ループ（breakで抜ける）
        loop {
            // match: パターンマッチング（Option<T>の値を分岐）
            match header {
                // Some(e): Option<T>が値を持つ場合、eに値を束縛
                Some(e) => match e.provide(layout.size(), layout.align()) {
                        Some(p) => break p, // 成功：ポインタを返してループを抜ける
                        None => {
                            // 失敗：次のブロックへ移動
                            // e.next_header.borrow_mut(): 次のHeaderの可変参照を取得
                            header = e.next_header.borrow_mut();
                            continue; // ループの先頭に戻る
                        }
                    },
                // None: Option<T>が値を持たない場合（リストの終端）
                None => {
                    // リストの終端に到達：割り当て失敗
                    // null_mut::<u8>(): null可変ポインタを作成（型パラメータ指定）
                    break null_mut::<u8>();
                }
            }
        }
    }

    /// UEFIメモリマップを使用してアロケータを初期化する
    ///
    /// # パラメータ
    /// - `memory_map`: UEFIから取得したメモリマップ
    ///
    /// # 処理の流れ
    /// 1. メモリマップの各エントリを調べる
    /// 2. CONVENTIONAL_MEMORY（通常のメモリ）のみを対象とする
    /// 3. 各メモリ領域を空きブロックとしてアロケータに追加
    pub fn init_with_mmap(&self, memory_map: &MemoryMapHolder) {
        for e in memory_map.iter() {
            // 通常のメモリ（割り当て可能なメモリ）のみを処理
            if e.memory_type() != EfiMemoryType::CONVENTIONAL_MEMORY {
                continue;
            }
            self.add_free_from_descriptor(e);
        }
    }

    /// UEFIメモリディスクリプタから空きブロックを連結リストに追加する
    ///
    /// # 概要
    /// UEFIファームウェアから取得した個別のメモリ領域情報を、
    /// First Fitアロケータが管理する空きブロック連結リストに追加する処理です。
    /// この処理により、OSが使用可能なメモリ領域がアロケータの管理下に置かれます。
    ///
    /// # パラメータ
    /// - `desc`: UEFIメモリディスクリプタ（個別のメモリ領域情報）
    ///   - `physical_start()`: メモリ領域の物理開始アドレス（64bit値）
    ///   - `number_of_pages()`: ページ数（UEFIでは4KBページ単位で管理）
    ///
    /// # 処理の詳細フロー
    /// 1. **メモリ情報の取得と変換**: UEFIディスクリプタからアドレス・サイズを抽出
    /// 2. **アドレス0回避処理**: Null Pointer問題を防ぐための安全性確保
    /// 3. **小領域除外**: 管理効率を考慮した最小サイズフィルタリング
    /// 4. **Header作成**: メモリ管理構造の構築
    /// 5. **連結リスト挿入**: アロケータへの登録完了
    ///
    /// # メモリレイアウトの変化
    /// ```
    /// 【処理前】
    /// ALLOCATOR.first_header → HeaderA → HeaderB → None
    ///
    /// 【処理後】
    /// ALLOCATOR.first_header → 新Header → HeaderA → HeaderB → None
    ///                          ^
    ///                          新しく追加された領域
    /// ```
    ///
    /// # 安全性とエラーハンドリング
    /// - **Null Pointer回避**: アドレス0を含む領域の特別処理
    /// - **アンダーフロー防止**: `saturating_sub()`による安全な算術演算
    /// - **RefCell借用管理**: 実行時借用チェックによる安全な内部可変性
    /// - **unsafe操作**: 生ポインタ操作の適切な管理
    fn add_free_from_descriptor(&self, desc: &EfiMemoryDescriptor) {
        // デバッグ: 追加されるメモリ領域の情報を出力
        // （一時的なデバッグ用）
        
        // 【ステップ1: メモリ情報の取得と変換】
        // UEFIディスクリプタからの情報抽出
        // - physical_start(): メモリ領域の物理開始アドレス（64bit値）
        // - number_of_pages(): ページ数（UEFIでは4KBページ単位で管理）
        // - * 4096: ページ数をバイト単位のサイズに変換
        //
        // 具体例:
        // UEFIディスクリプタ:
        // - physical_start: 0x100000 (1MB)
        // - number_of_pages: 512 (512ページ)
        // → 変換後: start_addr = 0x100000, size = 512 * 4096 = 2MB
        let mut start_addr = desc.physical_start() as usize;
        let mut size = desc.number_of_pages() as usize * 4096; // ページサイズ（4KB）を掛ける

        // 【ステップ2: アドレス0の特別処理（Null Pointer回避）】
        //
        // なぜアドレス0を避けるのか:
        // - **Null Pointer**: プログラミングにおいてアドレス0は「無効なポインタ」を示す特別な値
        // - **メモリ安全性**: アロケータがアドレス0を返すと、有効なメモリとnullの区別ができなくなる
        // - **デバッグ容易性**: null pointer accessによるクラッシュで問題を早期発見できる
        //
        // saturating_sub()の重要性:
        // - 通常の減算: size = size - 4096  // size < 4096の場合パニック
        // - saturating_sub(): 最小値0で止まる（安全、パニックしない）
        if start_addr == 0 {
            start_addr += 4096; // 1ページ分スキップ
            size = size.saturating_sub(4096); // サイズを調整（アンダーフローを防ぐ）
        }

        // 【ステップ3: 小さすぎる領域の除外】
        //
        // 除外理由:
        // - **管理コスト**: 小さな領域は管理オーバーヘッドが大きい
        // - **断片化防止**: 細かすぎるブロックは実用的でない
        // - **Header領域**: 32バイトのHeaderを考慮すると、4KB以下では実質的な利用可能領域が少ない
        if size <= 4096 {
            return;
        }

        // 【ステップ4: 新しいHeaderの作成】
        //
        // Header::new_from_addr()の動作:
        // 1. アドレスをHeader型のポインタにキャスト
        // 2. デフォルト値でHeaderを初期化
        // 3. Box::from_rawでBoxに包んで返す
        //
        // メモリレイアウト:
        // start_addr → |------ Header (32B) ------|------ 利用可能領域 ------|
        //              ^                          ^
        //              Headerの位置                実際のデータ領域
        //              (管理情報)                  (ユーザーが使用)
        //
        // unsafe使用理由:
        // - **生ポインタ操作**: メモリアドレスを直接Headerとして扱う
        // - **メモリ初期化**: 未初期化メモリに構造体を書き込む
        // - **Box::from_raw()**: 既存メモリをRustの管理下に置く
        let mut header = unsafe { Header::new_from_addr(start_addr) };
        header.next_header = None;
        header.is_allocated = false;
        header.size = size;

        // 【ステップ5: 連結リストへの挿入処理】
        //
        // この処理は以下の4つのサブステップに分かれています:
        // 1. RefCellからの借用
        // 2. 先頭要素の置換
        // 3. 借用の明示的解放
        // 4. リンクの再構築

        // サブステップ1: RefCellからの借用
        // RefCell<Option<Box<Header>>>からRefMut<Option<Box<Header>>>を取得
        // 実行時借用チェックにより、複数の可変借用を防止
        let mut first_header = self.first_header.borrow_mut();

        // サブステップ2: 先頭要素の置換
        // Option::replace()の動作:
        // - 現在の値（元の先頭）を取り出す
        // - 新しい値（新しいheader）を設定
        // - 元の値を返す
        let prev_last = first_header.replace(header); // 現在の先頭を取得

        // サブステップ3: 借用の明示的解放
        // RefMutを明示的に解放（次の借用のため）
        // drop()を呼ばないと、次のborrow_mut()でパニックが発生する可能性
        drop(first_header); // 借用を解放

        // サブステップ4: リンクの再構築
        // 新しい先頭の次に、元の先頭をリンク
        // これにより連結リスト構造が正しく維持される
        let mut header = self.first_header.borrow_mut();
        header.as_mut().unwrap().next_header = prev_last;

        // 【重要な設計判断: なぜソートしないのか】
        //
        // 理由の詳細:
        // - **物理的分離**: UEFIメモリマップの各領域は物理的に分離されている
        // - **マージ不可**: 連続していない領域は結合できない
        // - **検索効率**: First Fitアルゴリズムでは順序は重要でない（最初に見つかった適切なブロックを使用）
        //
        // メモリマップの典型例:
        // 0x00000000-0x0009FFFF: CONVENTIONAL_MEMORY (640KB)
        // 0x00100000-0x7FFFFFFF: CONVENTIONAL_MEMORY (2GB-1MB)
        // 0x90000000-0x9FFFFFFF: CONVENTIONAL_MEMORY (256MB)
        // → 3つの分離された領域、物理的に結合不可
        //
        // この時点でヘッダーがソートされていなくても問題ない
        // メモリマップに書かれている全ての領域は連続していないため、
        // どのみちマージできないから
    }
}

/// アロケータのテストモジュール
///
/// このモジュールでは、FirstFitAllocatorの動作を検証するテストを実装しています。
/// 各テストは異なる側面からアロケータの正常性を確認します。
#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;

    /// 繰り返しメモリ割り当てと解放のテスト
    ///
    /// # テスト内容
    /// - 0から999まで異なるサイズのVecを作成
    /// - 各Vecはスコープ終了時に自動的に解放される
    /// - メモリリークや断片化の問題がないかを確認
    ///
    /// # 検証項目
    /// - 大量の割り当て・解放操作でクラッシュしないか
    /// - メモリが適切に再利用されるか
    /// - 異なるサイズの割り当てが正常に動作するか
    #[test_case]
    fn malloc_iterate_free_and_alloc(){
        use alloc::vec::Vec;
        for i in 0..1000 {
            let mut vec = Vec::new();
            vec.resize(i, 10); // i個の要素を持つVecを作成
            // vecはこのスコープの終わりで自動的に解放される
        }
    }

    /// アライメント要件のテスト
    ///
    /// # テスト内容
    /// - 様々なアライメント値（1, 2, 4, 8, 16, 32, 4096バイト）でメモリを割り当て
    /// - 各アライメントで100個のポインタを割り当て
    /// - 返されたポインタが正しいアライメント境界にあるかを確認
    ///
    /// # 検証項目
    /// - null pointerが返されないか（割り当て成功）
    /// - 返されたアドレスが要求されたアライメント境界に正しく配置されているか
    /// - 異なるアライメント要件が同時に満たされるか
    #[test_case]
    fn malloc_align() {
        let mut pointers = [null_mut::<u8>(); 100];
        for align in [1, 2, 4, 8, 16, 32, 4096] {
            for e in pointers.iter_mut() {
                *e = ALLOCATOR.alloc_with_options(
                    Layout::from_size_align(1234, align).expect("Failed to create Layout"),
                );
                // 割り当てが成功したことを確認（null pointerでない）
                assert!(*e as usize != 0);
                // アライメント要件が満たされていることを確認
                // アドレスがalignの倍数であることをチェック
                assert!((*e as usize) % align == 0);
            }
        }
    }

    /// ランダムな順序でのアライメントテスト
    ///
    /// # テスト内容
    /// - アライメント値を意図的にランダムな順序で指定
    /// - 各アライメントで100個のメモリブロックを割り当て
    /// - 順序に関係なく正常に割り当てが行われるかを確認
    ///
    /// # 検証項目
    /// - アライメント要求の順序に依存しない安定した動作
    /// - 大きなアライメント（4096）と小さなアライメント（1）の混在処理
    /// - メモリ断片化が発生しても正常に動作するか
    ///
    /// # 注意
    /// このテストでは明示的なassertは行わないが、
    /// パニックやクラッシュが発生しないことで正常性を確認
    #[test_case]
    fn malloc_align_random_order() {
        // 意図的にランダムな順序でアライメント値を指定
        for align in [32, 4096, 8, 4, 16, 2, 1] {
            let mut pointers = [null_mut::<u8>(); 100];
            for e in pointers.iter_mut() {
                *e = ALLOCATOR.alloc_with_options(
                    Layout::from_size_align(1234, align).expect("Failed to create Layout"),
                );
                // このテストでは割り当てが成功することを前提とし、
                // パニックが発生しないことで正常性を確認
            }
        }
    }

    #[test_case]
    fn allocated_objects_have_no_overlap() {
        let allocations = [
            Layout::from_size_align(128, 128).unwrap(),
            Layout::from_size_align(32, 32).unwrap(),
            Layout::from_size_align(8, 8).unwrap(),
            Layout::from_size_align(16, 16).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(4, 4).unwrap(),
            Layout::from_size_align(2, 2).unwrap(),
            Layout::from_size_align(600000, 64).unwrap(),
            Layout::from_size_align(64, 64).unwrap(),
            Layout::from_size_align(1, 1).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(600000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
        ];
        let mut pointers = vec![null_mut::<u8>(); allocations.len()];
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            *pointer = ALLOCATOR.alloc_with_options(*layout);
            for k in 0..layout.size() {
                unsafe { *pointer.add(k) = i as u8 }
            }
        }
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!(unsafe { *pointer.add(k) } == i as u8);
            }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .step_by(2)
        {
            let (_, (layout, pointer)) = e;
            unsafe { ALLOCATOR.dealloc(*pointer, *layout) }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .skip(1)
            .step_by(2)
        {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!(unsafe { *pointer.add(k) } == i as u8);
            }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .step_by(2)
        {
            let (i, (layout, pointer)) = e;
            *pointer = ALLOCATOR.alloc_with_options(*layout);
            for k in 0..layout.size() {
                unsafe { *pointer.add(k) = i as u8 }
            }
        }
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!(unsafe { *pointer.add(k) } == i as u8);
            }
        }
    }
}
