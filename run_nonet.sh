#!/bin/sh

sudo qemu-system-aarch64 -M virt -cpu cortex-a53 -smp cpus=4 -m 1024M -display none -serial stdio -global virtio-mmio.force-legacy=false -device virtio-rng-device -drive if=none,cache=directsync,file=test.img,format=raw,id=hd0 -device virtio-blk-device,drive=hd0 -kernel $1
