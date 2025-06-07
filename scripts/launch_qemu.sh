#!/bin/bash -e
# スクリプトの絶対パスからプロジェクトルートを算出
PROJ_ROOT="$(dirname $(dirname ${BASH_SOURCE[0]:-scripts/launch_qemu.sh}))"
cd "${PROJ_ROOT}"

# 第1引数がEFIバイナリパス
PATH_TO_EFI="$1"
rm -rf mnt
mkdir -p mnt/EFI/BOOT/
cp ${PATH_TO_EFI} mnt/EFI/BOOT/BOOTX64.EFI
set +e
qemu-system-x86_64 \
    -m 4G \
    -bios third_party/ovmf/RELEASEX64_OVMF.fd \
    -drive format=raw,file=fat:rw:mnt \
    -device isa-debug-exit,iobase=0xf4,iosize=0x01
RETCODE=$?
set -e
if [.$RETCODE -eq 0 ]; then
exit 0
elif [.$RETCODE -eq 3 ]; thenprintf "\nPASS\n"
exit 0
else
printf "\nFAIL: QEMU returned $RETCODE\n"
exit 1
fi
