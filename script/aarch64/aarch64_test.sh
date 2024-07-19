#!/bin/bash

echo "ARGS1 $1"

rust-objcopy --binary-architecture=aarch64 $1 --strip-all -O binary $1.bin

qemu-system-aarch64 \
    -cpu cortex-a57 \
    -machine virt \
    -kernel $1.bin \
    -nographic -smp 1 \
    -D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors