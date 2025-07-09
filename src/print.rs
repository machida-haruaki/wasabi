use crate::serial::SerialPort;
use core::fmt;
use core::mem::size_of;
use core::slice;

pub fn global_print(args: fmt::Arguments) {
    let mut writer = SerialPort::default();
    fmt::write(&mut writer, args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::global_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules!  println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n",format_args!($($arg)*)));
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::print!("[INFO] {}:{:<3}: {}\n",
                        file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::print!("[WARN] {}:{:<3}: {}\n",
                        file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::print!("[ERROR] {}:{:<3}: {}\n",
                        file!(), line!(), format_args!($($arg)*)));
}

/// バイト配列をhexdump形式で出力する内部関数
///
/// # 引数
/// * `bytes` - &[u8]: バイトスライスへの参照（借用）
///   スライス(&[u8])は配列の一部または全体への参照で、サイズが実行時に決まる
fn hexdump_bytes(bytes: &[u8]) {
    // 現在の行での位置を追跡（0-15の範囲）
    let mut i = 0;

    // 16バイト分のASCII文字を保存するバッファ
    // [0u8; 16] は「u8型の0で初期化された16要素の配列」を意味
    let mut ascii = [0u8; 16];

    // 現在のメモリオフセット（各行の開始アドレス）
    let mut offset = 0;

    // bytes.iter() でバイトスライスのイテレータを作成
    // for文でイテレータの各要素を順番に処理
    for v in bytes.iter() {
        // 新しい行の開始時（i == 0）にオフセットを表示
        if i == 0 {
            // {:08x} は「8桁の16進数、不足分は0で埋める」という意味
            print!("{offset:08x}: ");
        }

        // 各バイトを2桁の16進数で表示
        // {:02x} は「2桁の16進数、不足分は0で埋める」という意味
        print!("{:02x}", v);

        // ASCII表示用にバイト値を保存
        // *v でイテレータから実際の値を取得（参照外し）
        ascii[i] = *v;

        // 位置カウンタを進める
        i += 1;

        // 16バイト処理したら行を完了
        if i == 16 {
            // ASCII部分の開始を示す区切り文字
            print!("|");

            // ASCII配列の各バイトを文字として表示
            for c in ascii.iter() {
                print!(
                    "{}",
                    // match式：値に応じて異なる処理を実行
                    match c {
                        // 0x20..=0x7e は範囲パターン（スペース～チルダ）
                        // 印刷可能なASCII文字の範囲
                        0x20..=0x7e => {
                            // *c as char でu8をchar型にキャスト
                            *c as char
                        }
                        // _ はワイルドカードパターン（その他すべて）
                        _ => {
                            // 印刷不可能な文字は'.'で表示
                            '.'
                        }
                    }
                );
            }
            // ASCII部分の終了を示す区切り文字と改行
            println!("|");

            // 次の行のためにオフセットを16バイト進める
            offset += 16;
            // 行内位置をリセット
            i = 0;
        }
    }

    // 最後の行が16バイトに満たない場合の処理
    if i != 0 {
        // 現在の位置を保存（ASCII表示で使用）
        let old_i = i;

        // 16バイトになるまで空白で埋める
        while i < 16 {
            // "   " は16進数2桁分のスペース
            print!("  ");
            i += 1;
        }

        // ASCII部分の開始
        print!("|");

        // 実際に存在するバイト分だけASCII表示
        // ascii[0..old_i] はスライス記法：0からold_i-1までの要素
        for c in ascii[0..old_i].iter() {
            print!(
                "{}",
                // contains()メソッドで範囲内かチェック
                // (0x20u8..=0x7fu8) は範囲オブジェクト
                if (0x20u8..=0x7fu8).contains(c) {
                    *c as char
                } else {
                    '.'
                }
            );
        }
        println!("|");
    }
}

/// 任意の型のデータをhexdump形式で出力する公開関数
///
/// # ジェネリクス
/// * `T: Sized` - 型パラメータTはSizedトレイトを実装する必要がある
///   Sizedは「コンパイル時にサイズが決まる型」を意味する
///
/// # 引数
/// * `data` - &T: 型Tへの参照（借用）
///
/// # 使用例
/// ```
/// let value = 0x12345678u32;
/// hexdump(&value);  // u32のメモリ表現を表示
///
/// let array = [1u8, 2, 3, 4];
/// hexdump(&array);  // 配列のメモリ表現を表示
/// ```
pub fn hexdump<T: Sized>(data: &T) {
    // unsafeブロック：メモリ安全性の保証をプログラマが責任を持つ
    // 生ポインタを扱うため、Rustの借用チェッカーを迂回する必要がある
    hexdump_bytes(unsafe {
        // 複雑なポインタ変換を段階的に説明：
        // 1. data as *const T: 参照&Tを生ポインタ*const Tに変換
        // 2. as *const u8: 型Tのポインタをu8のポインタに変換（バイト単位でアクセスするため）
        // 3. size_of::<T>(): 型Tのサイズをバイト数で取得（コンパイル時に決定）
        // 4. slice::from_raw_parts(): 生ポインタとサイズからスライス&[u8]を作成
        //
        // この処理により、任意の型のメモリ領域をバイト配列として扱える
        slice::from_raw_parts(data as *const T as *const u8, size_of::<T>())
    })
}
