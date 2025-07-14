// -----------------------------------------
// 1. グローバル設定
// -----------------------------------------
#![no_std] // OSの標準ライブラリを使わない
#![no_main] // 通常のmain関数を使わず、UEFI用のエントリポイントを定義
#![feature(offset_of)] // フィールドのオフセット取得を有効にする

// -----------------------------------------
// 2. コアライブラリのインポート
// -----------------------------------------
use core::fmt::Write;
use core::panic::PanicInfo;
use core::writeln;
use wasabi::error;
use wasabi::graphics::draw_test_pattern;
use wasabi::graphics::fill_rect;
use wasabi::graphics::Bitmap;
use wasabi::info;
use wasabi::print::hexdump;
use wasabi::init::init_basic_runtime;
use wasabi::println;
use wasabi::qemu::exit_qemu;
use wasabi::qemu::QemuExitCode;
use wasabi::uefi::init_vram;
use wasabi::uefi::EfiHandle;
use wasabi::uefi::EfiMemoryType;
use wasabi::uefi::EfiSystemTable;
use wasabi::uefi::VramTextWriter;
use wasabi::warn;
use wasabi::x86::hlt;

// -----------------------------------------
// 13. エントリポイント: efi_main
// -----------------------------------------
#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    println!("Booting WasabiOS...");
    println!("image/handle: {:#018x}", image_handle);
    println!("efi_system_table: {:#p}", efi_system_table);
    info!("info");
    warn!("warn");
    error!("error");
    hexdump(efi_system_table);
    let mut vram = init_vram(efi_system_table).expect("init_vram failed");
    let vw = vram.width();
    let vh = vram.height();
    fill_rect(&mut vram, 0x000000, 0, 0, vw, vh).expect("fill_rect failed");

    draw_test_pattern(&mut vram);
    let mut w = VramTextWriter::new(&mut vram);

    let memory_map = init_basic_runtime(image_handle, efi_system_table);
    let mut total_memory_pages = 0;
    for e in memory_map.iter() {
        if e.memory_type() != EfiMemoryType::CONVENTIONAL_MEMORY {
            continue;
        }
        total_memory_pages += e.number_of_pages();
        writeln!(w, "{e:?}").unwrap();
    }
    let total_memory_size_mib = total_memory_pages * 4096 / 1024 / 1024;
    writeln!(
        w,
        "Total: {total_memory_pages} pages = {total_memory_size_mib} Mib"
    )
    .unwrap();

    writeln!(w, "Hello, Non-UEFI world!").unwrap();
    let cr3 = wasabi::x86::read_cr3();
    println!("cr3 = {cr3:#p}");
    hexdump(unsafe { &*cr3 });
    loop {
        hlt()
    }
}

// -----------------------------------------
// 15. パニックハンドラ
// -----------------------------------------
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {info:?}");
    exit_qemu(QemuExitCode::Fail);
}
