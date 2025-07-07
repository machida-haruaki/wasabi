#![no_std]
#![feature(offset_of)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner::test_runner)]
#![reexport_test_harness_main = "run_unit_tests"]
#![no_main]
pub mod allocator;
pub mod graphics;
pub mod qemu;
pub mod result;
pub mod serial;
pub mod uefi;
pub mod x86;

#[cfg(test)]
pub mod test_runner;

#[cfg(test)]
#[no_mangle]
fn efi_main(
    image_handle: uefi::EfiHandle,
    efi_system_table: &uefi::EfiSystemTable,
) {
    let mut memory_map = uefi::MemoryMapHolder::new();
    uefi::exit_from_efi_boot_services(
        image_handle,
        efi_system_table,
        &mut memory_map,
    );

    // デバッグ: メモリマップの内容を出力
    let mut sw = serial::SerialPort::new_for_com1();
    use core::fmt::Write;
    writeln!(sw, "=== Memory Map Debug ===").unwrap();
    for e in memory_map.iter() {
        if e.memory_type() == uefi::EfiMemoryType::CONVENTIONAL_MEMORY {
            writeln!(sw, "CONVENTIONAL: start={:#x}, pages={}, size={:#x}", 
                     e.physical_start(), e.number_of_pages(), e.number_of_pages() * 4096).unwrap();
        }
    }
    writeln!(sw, "=== End Memory Map ===").unwrap();

    allocator::ALLOCATOR.init_with_mmap(&memory_map);
    run_unit_tests()
}
