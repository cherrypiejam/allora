#!/bin/sh

qemu-system-aarch64 \
    -M raspi3b \
    -m 1024M \
    -nographic \
    -drive if=none,cache=directsync,file=test.img,format=raw,id=hd0 \
    -serial null -serial pty \
    -kernel $1 \
    -usb -device usb-mouse -device usb-kbd \
         -device usb-net,netdev=net0 \
         -netdev user,id=net0,hostfwd=tcp::5555-:22 \

    # -serial stdio \
    # -dtb bcm2710-rpi-3-b.dtb \
    # -drive if=none,cache=directsync,file=test.img,format=raw,id=hd0 \
    # -sd test.img \

# qemu-system-aarch64 -M raspi3b -cpu cortex-a53 -smp cpus=4 -m 1024M -display none -serial stdio -global virtio-mmio.force-legacy=false -device virtio-rng-device -drive if=none,cache=directsync,file=test.img,format=raw,id=hd0 -device virtio-blk-device,drive=hd0 -kernel $1

echo $1

