ARCH := riscv64
TARGET := riscv64imac-unknown-none-elf
ifeq ($(ARCH), riscv64)
	TARGET := riscv64imac-unknown-none-elf
else ifeq ($(ARCH), aarch64)
	TARGET := aarch64-unknown-none-softfloat
else
	@echo "Unknown target arch: $(ARCH)"
endif

test_build:
	@python3 build.py

print-%:
	@echo $*=$($*)

test: test_build
	@cargo test -F unit-test

.PHONY: clean test
clean:
	rm -rf .cargo src/entry.asm *.log linker.ld