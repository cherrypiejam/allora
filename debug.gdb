set disassemble-next-line on
set confirm off
add-symbol-file target/aarch64-unknown-none/debug/aarch64os
target remote tcp::1234
set arch aarch64
layout regs
