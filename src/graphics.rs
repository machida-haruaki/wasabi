use crate::result::Result;
use core::cmp::min;

// -----------------------------------------
// 8. VRAM操作の抽象化: Bitmapトレイト
// -----------------------------------------
pub trait Bitmap {
    fn bytes_per_pixel(&self) -> i64;
    fn pixels_per_line(&self) -> i64;
    fn width(&self) -> i64;
    fn height(&self) -> i64;
    fn buf_mut(&mut self) -> *mut u8;

    /// # Safety
    ///
    /// Returned pointer is valid as long as the given coordinates are valid
    /// which means that passing is_in_*_range tests.
    unsafe fn unchecked_pixel_at_mut(&mut self, x: i64, y: i64) -> *mut u32 {
        self.buf_mut()
            .add(((y * self.pixels_per_line() + x) * self.bytes_per_pixel()) as usize)
            as *mut u32
    }

    fn pixel_at_mut(&mut self, x: i64, y: i64) -> Option<&mut u32> {
        if self.is_in_x_range(x) && self.is_in_y_range(y) {
            unsafe { Some(&mut *(self.unchecked_pixel_at_mut(x, y))) }
        } else {
            None
        }
    }

    fn is_in_x_range(&self, px: i64) -> bool {
        0 <= px && px < min(self.width(), self.pixels_per_line())
    }

    fn is_in_y_range(&self, py: i64) -> bool {
        0 <= py && py < self.height()
    }
}

// -----------------------------------------
// 12. ピクセル/矩形/線の描画系関数
// -----------------------------------------

unsafe fn unchecked_draw_point<T: Bitmap>(buf: &mut T, color: u32, x: i64, y: i64) {
    *buf.unchecked_pixel_at_mut(x, y) = color;
}

fn draw_point<T: Bitmap>(buf: &mut T, color: u32, x: i64, y: i64) -> Result<()> {
    *(buf.pixel_at_mut(x, y).ok_or("Out of Range")?) = color;
    Ok(())
}

pub fn fill_rect<T: Bitmap>(
    buf: &mut T,
    color: u32,
    px: i64,
    py: i64,
    w: i64,
    h: i64,
) -> Result<()> {
    if !buf.is_in_x_range(px)
        || !buf.is_in_y_range(py)
        || !buf.is_in_x_range(px + w - 1)
        || !buf.is_in_y_range(py + h - 1)
    {
        return Err("Out of Range");
    }

    for y in py..py + h {
        for x in px..px + w {
            unsafe { unchecked_draw_point(buf, color, x, y) }
        }
    }
    Ok(())
}

/// 📌 新規追加: 線分描画の補完計算を行う補助関数
fn calc_slope_point(da: i64, db: i64, ia: i64) -> Option<i64> {
    if da < db {
        None
    } else if da == 0 {
        Some(0)
    } else if (0..=da).contains(&ia) {
        Some((2 * db * ia + da) / da / 2)
    } else {
        None
    }
}

/// 📌 新規追加: 任意の2点間を直線でつなぐ描画関数（ラスタライズ線分）
fn draw_line<T: Bitmap>(buf: &mut T, color: u32, x0: i64, y0: i64, x1: i64, y1: i64) -> Result<()> {
    if !buf.is_in_x_range(x0)
        || !buf.is_in_y_range(x1)
        || !buf.is_in_x_range(y0)
        || !buf.is_in_y_range(y1)
    {
        return Err("Out of Range");
    }

    let dx = (x1 - x0).abs();
    let sx = (x1 - x0).signum();
    let dy = (y1 - y0).abs();
    let sy = (y1 - y0).signum();

    if dx >= dy {
        for (rx, ry) in (0..dx).flat_map(|rx| calc_slope_point(dx, dy, rx).map(|ry| (rx, ry))) {
            draw_point(buf, color, x0 + rx * sx, y0 + ry * sy)?;
        }
    } else {
        for (rx, ry) in (0..dy).flat_map(|ry| calc_slope_point(dy, dx, ry).map(|rx| (rx, ry))) {
            draw_point(buf, color, x0 + rx * sx, y0 + ry * sy)?;
        }
    }

    Ok(())
}

fn lookup_font(c: char) -> Option<[[char; 8]; 16]> {
    // コンパイル時にfont.txtの内容を文字列として埋め込む
    const FONT_SOURCE: &str = include_str!("./font.txt");

    // 静的な可変変数でフォントキャッシュを宣言
    // Option<T>: Rustの標準ライブラリで定義されている列挙型
    //   enum Option<T> {
    //       None,        // 値が存在しない
    //       Some(T),     // 値Tが存在する
    //   }
    // [[[char; 8]; 16]; 256]: 3次元配列の型
    //   - 最外層[256]: ASCII文字256文字分のスロット
    //   - 中間層[16]: 各文字の高さ16ピクセル分の行
    //   - 最内層[8]: 各行の幅8文字分
    // static mut: プログラム全体で共有される可変なグローバル変数
    // = None: 初期値としてNone（未初期化状態）を設定
    static mut FONT_CACHE: Option<[[[char; 8]; 16]; 256]> = None;

    // char型をu8型に変換を試行（ASCII範囲内かチェック）
    // try_from: 失敗する可能性のある型変換（Result<u8, Error>を返す）
    // if let Ok(c): 変換が成功した場合のみ処理を続行
    if let Ok(c) = u8::try_from(c) {
        // unsafeブロック: static mut変数へのアクセスには必要
        // Rustは複数スレッドからの同時アクセスを防げないため、開発者が安全性を保証する必要がある
        let font = unsafe {
            // get_or_insert_with: Option<T>型に実装されているメソッド
            // 動作：
            //   - FONT_CACHEがNoneの場合：クロージャ || { ... } を実行して初期化し、その結果をSome()で包んで格納
            //   - FONT_CACHEが既にSome(値)の場合：既存の値への参照をそのまま返す（キャッシュヒット）
            // 戻り値：&mut T （今回の場合は &mut [[[char; 8]; 16]; 256]）
            FONT_CACHE.get_or_insert_with(|| {
                // 全256文字分のフォントデータを格納する3次元配列を初期化
                // ['*'; 8]: 8個の'*'文字で初期化された配列
                // [['*'; 8]; 16]: 上記の配列を16個持つ配列（16行分）
                // [[['*'; 8]; 16]; 256]: 上記を256個持つ配列（256文字分）
                let mut font = [[['*'; 8]; 16]; 256];

                // FONT_SOURCEを改行文字で分割してイテレータを作成
                // split('\n'): 文字列を改行で分割
                let mut fi = FONT_SOURCE.split('\n');

                // 各行を順次処理
                // while let Some(line): イテレータから次の要素を取得、なくなったらループ終了
                while let Some(line) = fi.next() {
                    // strip_prefix("0x"): 行の先頭が"0x"で始まる場合、それを除去した残りを返す
                    // if let Some(line): strip_prefixが成功した場合のみ処理続行
                    if let Some(line) = line.strip_prefix("0x") {
                        // 16進数文字列をu8に変換
                        // from_str_radix(文字列, 基数): 指定した基数で文字列を数値に変換
                        // 16: 16進数として解釈
                        if let Ok(idx) = u8::from_str_radix(line, 16) {
                            // 1文字分のグリフ（字形）データを格納する配列
                            // [['*'; 8]; 16]: 16行×8列の2次元配列
                            let mut glyph = [['*'; 8]; 16];

                            // 現在の文字の16行分のフォントデータを読み取り
                            // fi.clone(): イテレータを複製（元のイテレータは保持）
                            // take(16): 最大16個の要素のみ取得
                            // enumerate(): (インデックス, 値) のタプルを生成
                            for (y, line) in fi.clone().take(16).enumerate() {
                                // 各行の文字を1文字ずつ処理
                                // line.chars(): 文字列を文字のイテレータに変換
                                // enumerate(): (x座標, 文字) のタプルを生成
                                for (x, c) in line.chars().enumerate() {
                                    // 配列の境界チェック付きで要素への可変参照を取得
                                    // get_mut(x): x番目の要素への可変参照をOption<&mut T>で返す
                                    // 範囲外アクセスの場合はNoneを返す
                                    if let Some(e) = glyph[y].get_mut(x) {
                                        // 参照先の値を更新
                                        // *e: 参照を逆参照して実際の値にアクセス
                                        *e = c;
                                    }
                                }
                            }
                            // 完成したグリフデータをフォント配列の適切な位置に格納
                            // idx as usize: u8をusizeにキャスト（配列インデックスに使用）
                            font[idx as usize] = glyph;
                        }
                    }
                }
                // 初期化完了したフォント配列を返す
                // この値がFONT_CACHEのSome()に格納される
                font
            })
        };
        // キャッシュから指定された文字のフォントデータを取得して返す
        // c as usize: u8をusizeにキャスト
        // font[c as usize]: 配列への直接アクセス（O(1)の高速アクセス）
        Some(font[c as usize])
    } else {
        // char型からu8型への変換が失敗した場合（ASCII範囲外の文字）
        // None: フォントデータが存在しないことを示す
        None
    }
}

pub fn draw_font_fg<T: Bitmap>(buf: &mut T, x: i64, y: i64, color: u32, c: char) {
    if let Some(font) = lookup_font(c) {
        for (dy, row) in font.iter().enumerate() {
            for (dx, pixel) in row.iter().enumerate() {
                let color = match pixel {
                    '*' => color,
                    _ => continue,
                };
                let _ = draw_point(buf, color, x + dx as i64, y + dy as i64);
            }
        }
    }
}

pub fn draw_str_fg<T: Bitmap>(buf: &mut T, x: i64, y: i64, color: u32, s: &str) {
    for (i, c) in s.chars().enumerate() {
        draw_font_fg(buf, x + i as i64 * 8, y, color, c)
    }
}

pub fn draw_test_pattern<T: Bitmap>(buf: &mut T) {
    let w = 128;
    let left = buf.width() - w - 1;
    let colors = [0x000000, 0xff0000, 0x00ff00, 0x0000ff];
    let h = 64;
    for (i, c) in colors.iter().enumerate() {
        let y = i as i64 * h;
        fill_rect(buf, *c, left, y, h, h).expect("fill_rect failed");
        fill_rect(buf, !*c, left + h, y, h, h).expect("fill_rect failed");
    }
    let points = [(0, 0), (0, w), (w, 0), (w, w)];
    for (x0, y0) in points.iter() {
        for (x1, y1) in points.iter() {
            let _ = draw_line(buf, 0xffffff, left + *x0, *y0, left + *x1, *y1);
        }
    }
    draw_str_fg(buf, left, h * colors.len() as i64, 0x00ff00, "0123456789");
    draw_str_fg(buf, left, h * colors.len() as i64 + 16, 0x00ff00, "ABCDEF");
}
