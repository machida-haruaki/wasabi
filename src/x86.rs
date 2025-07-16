//! # x86アーキテクチャ固有の低レベル操作
//!
//! このモジュールは、x86/x86_64アーキテクチャの低レベルなCPU命令や
//! I/Oポート操作を提供します。これらの関数は主にOSカーネルの
//! ハードウェア制御に使用されます。

use crate::result::Result;
use core::arch::asm;
use core::fmt;
use core::marker::PhantomData;

/// CPUを停止状態にする（HLT命令）
///
/// # 動作
/// - CPUを低電力の停止状態にする
/// - 割り込みが発生するまでCPUは停止したままになる
/// - 割り込み発生時に自動的に実行を再開する
///
/// # 用途
/// - アイドル状態での電力消費を削減
/// - 割り込み待ちの際の効率的な待機
/// - OSのスケジューラでのCPU休止処理
///
/// # 安全性
/// この関数は特権命令を使用するため、カーネルモードでのみ実行可能
pub fn hlt() {
    unsafe { asm!("hlt") }
}

/// ビジーループでのCPU最適化ヒント（PAUSE命令）
///
/// # 動作
/// - スピンロックやビジーウェイト中にCPUに最適化のヒントを提供
/// - CPUのパイプラインを最適化し、電力消費を削減
/// - 他のハイパースレッドに実行機会を与える
///
/// # 用途
/// - スピンロック実装での効率化
/// - ポーリング処理での最適化
/// - マルチスレッド環境でのCPU使用率改善
///
/// # 使用例
/// ```rust
/// // スピンロックの実装例
/// while !try_acquire_lock() {
///     busy_loop_hint(); // CPUに最適化ヒントを提供
/// }
/// ```
pub fn busy_loop_hint() {
    unsafe { asm!("pause") }
}

/// I/Oポートから8ビットデータを読み取る（IN命令）
///
/// # パラメータ
/// - `port`: 読み取り対象のI/Oポート番号（0-65535）
///
/// # 戻り値
/// - 指定されたポートから読み取った8ビットデータ
///
/// # 動作
/// - x86のIN命令を使用してI/Oポートからデータを読み取る
/// - ハードウェアデバイスとの直接通信に使用
///
/// # 用途
/// - シリアルポート、パラレルポートの制御
/// - PIC（割り込みコントローラ）の操作
/// - その他のレガシーハードウェアとの通信
///
/// # 安全性
/// - 不正なポート番号への アクセスはシステムクラッシュの原因となる可能性
/// - ハードウェアの仕様を正確に理解した上で使用すること
pub fn read_io_port_u8(port: u16) -> u8 {
    let mut data: u8;
    unsafe {
        // 実行順序：
        // 1. port → DXレジスタ（入力制約）
        // 2. IN AL, DX 命令実行
        // 3. ALレジスタ → data（出力制約）
        asm!(
            "in al, dx",                    // I/Oポートから8ビット読み取り
            in("dx") port,                  // 入力：ポート番号をDXレジスタに設定
            out("al") data                  // 出力：読み取り結果をALレジスタから取得
        );
    }
    data
}

/// I/Oポートに8ビットデータを書き込む（OUT命令）
///
/// # パラメータ
/// - `port`: 書き込み対象のI/Oポート番号（0-65535）
/// - `data`: 書き込む8ビットデータ
///
/// # 動作
/// - x86のOUT命令を使用してI/Oポートにデータを書き込む
/// - ハードウェアデバイスの制御や設定に使用
///
/// # 用途
/// - シリアルポートへのデータ送信
/// - ハードウェアレジスタの設定
/// - デバイスの制御コマンド送信
///
/// # 安全性
/// - 不正なポート番号やデータの書き込みはハードウェア障害の原因となる可能性
/// - デバイスの仕様書を参照して正しい値を書き込むこと
///
/// # 使用例
/// ```rust
/// // シリアルポート（COM1）にデータを送信する例
/// const COM1_DATA_PORT: u16 = 0x3F8;
/// write_io_port_u8(COM1_DATA_PORT, b'H'); // 'H'文字を送信
/// ```
pub fn write_io_port_u8(port: u16, data: u8) {
    unsafe {
        // 実行順序：
        // 1. data → ALレジスタ、port → DXレジスタ（入力制約）
        // 2. OUT DX, AL 命令実行
        asm!(
            "out dx, al",                   // I/Oポートに8ビット書き込み
            in("al") data,                  // 入力：書き込みデータをALレジスタに設定
            in("dx") port                   // 入力：ポート番号をDXレジスタに設定
        );
    }
}

pub fn read_cr3() -> *mut PML4 {
    let mut cr3: *mut PML4;
    unsafe {
        asm!("mov rax, cr3",
        out("rax") cr3)
    }
    cr3
}

pub const PAGE_SIZE: usize = 4096;
const ATTR_MASK: u64 = 0xFFF;
const ATTR_PRESENT: u64 = 1 << 0;
const ATTR_WRITABLE: u64 = 1 << 1;
const ATTR_WRITE_THROUGH: u64 = 1 << 3;
const ATTR_CACHE_DISABLE: u64 = 1 << 4;

#[derive(Debug, Copy, Clone)]
#[repr(u64)]
pub enum PageAttr {
    NotPresent = 0,
    ReadWriteKernel = ATTR_PRESENT | ATTR_WRITABLE,
    ReadWriteIo = ATTR_PRESENT | ATTR_WRITABLE | ATTR_WRITE_THROUGH | ATTR_CACHE_DISABLE,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TranslationResult {
    PageMapped4K { phys: u64 },
    PageMapped2M { phys: u64 },
    PageMapped1G { phys: u64 },
}

#[repr(transparent)]
pub struct Entry<const LEVEL: usize, const SHIFT: usize, NEXT> {
    value: u64,
    next_type: PhantomData<NEXT>,
}
impl<const LEVEL: usize, const SHIFT: usize, NEXT> Entry<LEVEL, SHIFT, NEXT> {
    fn read_value(&self) -> u64 {
        self.value
    }
    fn is_present(&self) -> bool {
        (self.read_value() & (1 << 0)) != 0
    }
    fn is_writable(&self) -> bool {
        (self.read_value() & (1 << 1)) != 0
    }
    fn is_user(&self) -> bool {
        (self.read_value() & (1 << 2)) != 0
    }
    fn format(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "L{}Entry @ {:#p} {{ {:#018x} {}{}{}",
            LEVEL,
            self,
            self.read_value(),
            if self.is_present() { "P" } else { "N" },
            if self.is_writable() { "W" } else { "R" },
            if self.is_user() { "U" } else { "S" }
        )?;
        write!(f, "}}")
    }
    fn table(&self) -> Result<&NEXT> {
        if self.is_present() {
            Ok(unsafe { &*((self.value & !ATTR_MASK) as *const NEXT ) })
        } else {
            Err("Page Not Found")
        }
    }
}
impl<const LEVEL: usize, const SHIFT: usize, NEXT> fmt::Display for Entry<LEVEL, SHIFT, NEXT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}
impl<const LEVEL: usize, const SHIFT: usize, NEXT> fmt::Debug for Entry<LEVEL, SHIFT, NEXT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}

#[repr(align(4096))]
pub struct Table<const LEVEL: usize, const SHIFT: usize, NEXT> {
    entry: [Entry<LEVEL, SHIFT, NEXT>; 512],
}
impl<const LEVEL: usize, const SHIFT: usize, NEXT: core::fmt::Debug> Table<LEVEL, SHIFT, NEXT> {
    fn format(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "L{}Table @ {:#p} {{", LEVEL, self)?;
        for i in 0..512 {
            let e = &self.entry[i];
            if !e.is_present() {
                continue;
            }
            writeln!(f, " entry[{:3}] = {:?}", i, e)?;
        }
        writeln!(f,"}}")
    }
    pub fn next_level(&self, index: usize) -> Option<&NEXT> {
        self.entry.get(index).and_then(|e| e.table().ok())
    }
}
impl<const LEVEL: usize, const SHIFT: usize, NEXT: fmt::Debug> fmt::Debug for Table<LEVEL, SHIFT, NEXT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format(f)
    }
}

pub type PT = Table<1, 12, [u8; PAGE_SIZE]>;
pub type PD = Table<2, 21, PT>;
pub type PDPT = Table<3, 30, PD>;
pub type PML4 = Table<4, 39, PDPT>;
