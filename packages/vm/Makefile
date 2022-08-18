.SUFFIXES: # disable builtin make rules
MAKEFLAGS := --jobs=$(shell nproc) --no-builtin-rules

ROOT_DIR=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
TARGET_DIR=$(abspath $ROOT_DIR/../../../../../target)

CURL=curl -s

# Build qemu, and download the ubuntu image.
.PHONY: all
all: boot_vm template/debian template/fedora

# Build an example VM template
template/debian: \
	./download/debian/debian-12-generic-arm64-daily-20220513-1014.raw \
	$(TARGET_DIR)/aarch64-unknown-linux-gnu/release/guest_agent \
	linux/arch/arm64/boot/Image \
	examples/vm.toml
	rm -rf $@ && mkdir -p $@
	cp download/debian/debian-12-generic-arm64-daily-20220513-1014.raw $@
	cp $(TARGET_DIR)/aarch64-unknown-linux-gnu/release/guest_agent $@
	cp linux/arch/arm64/boot/Image $@
	ln -s ../debian.toml $@/vm.toml

template/fedora: \
	download/fedora/Fedora-Cloud-Base-36-1.5.aarch64.qcow2 \
	download/fedora/vmlinuz \
	download/fedora/initrd.img \
	$(TARGET_DIR)/aarch64-unknown-linux-gnu/release/guest_agent \
	examples/vm.toml
	rm -rf $@ && mkdir -p $@
	cp ./download/fedora/{Fedora-Cloud-Base-36-1.5.aarch64.qcow2,initrd.img} $@
	gzcat ./download/fedora/vmlinuz >$@/vmlinux
	cp $(TARGET_DIR)/aarch64-unknown-linux-gnu/release/guest_agent $@
	ln -s ../fedora.toml $@/vm.toml

# Build and codesign the Apple Virtualization.framework example
.PHONY: boot_vm
boot_vm: $(TARGET_DIR)/debug/examples/boot_vm_signed 
	@echo
	@echo "To run 'boot_vm':"
	@echo 'alias boot_vm="$(TARGET_DIR)/debug/examples/boot_vm_signed"'
	@echo 'alias lldb_boot_vm="lldb -o run $(TARGET_DIR)/debug/examples/boot_vm_signed"'
$(TARGET_DIR)/debug/examples/boot_vm_signed: $(shell find src examples)
	rm -rf $(TARGET_DIR)/debug/examples/boot_vm*
	cargo build --example boot_vm
	echo "$(ROOT_DIR)"
	codesign -f --entitlement examples/boot_vm.entitlements -s - $(TARGET_DIR)/debug/examples/boot_vm
	cp $(TARGET_DIR)/debug/examples/boot_vm $(TARGET_DIR)/debug/examples/boot_vm_signed

# To install the toolchain, run:
# $ brew tap messense/macos-cross-toolchains
# $ brew install aarch64-unknown-linux-gnu
.PHONY: guest_agent
guest_agent: $(TARGET_DIR)/aarch64-unknown-linux-gnu/release/guest_agent
$(TARGET_DIR)/aarch64-unknown-linux-gnu/release/guest_agent: $(shell find src/agent) src/bin/guest_agent.rs
	export CC_aarch64_unknown_linux_gnu=aarch64-unknown-linux-gnu-gcc && \
		export CXX_aarch64_unknown_linux_gnu=aarch64-unknown-linux-gnu-g++ && \
		export AR_aarch64_unknown_linux_gnu=aarch64-unknown-linux-gnu-ar && \
		export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-unknown-linux-gnu-gcc && \
		cargo build --release --bin guest_agent --target aarch64-unknown-linux-gnu

# Build linux with a configuration amenable to virtualization
linux: 
	git clone --depth 1 "git://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git" -b v5.18 linux
linux/.config:
	ln -s '../linux_configs/aarch64' linux/.config
linux/arch/arm64/boot/Image: linux linux/.config
	[ "$$(uname)" != "Darwin" ] # Can't build Linux on macOS
	cd linux && $(MAKE) Image


# Download and convert OS image files
download/debian/debian-12-generic-arm64-daily-20220513-1014.qcow2:
	mkdir -p $$(dirname $@)
	$(CURL) -o $@ -L \
		"https://cloud.debian.org/images/cloud/bookworm/daily/20220513-1014/debian-12-generic-arm64-daily-20220513-1014.qcow2"
download/fedora/Fedora-Cloud-Base-36-1.5.aarch64.qcow2:
	mkdir -p $$(dirname $@)
	$(CURL) -o $@ -L \
		"https://fedora.mirror.constant.com/fedora/linux/releases/36/Cloud/aarch64/images/Fedora-Cloud-Base-36-1.5.aarch64.qcow2"
download/fedora/vmlinuz:
	mkdir -p $$(dirname $@)
	$(CURL) -o $@ -L \
		"http://mirror.facebook.net/fedora/linux/releases/36/Everything/aarch64/os/images/pxeboot/vmlinuz"
download/fedora/initrd.img:
	mkdir -p $$(dirname $@)
	$(CURL) -o $@ -L \
		"http://mirror.facebook.net/fedora/linux/releases/36/Everything/aarch64/os/images/pxeboot/initrd.img"



%.raw: %.qcow2
	qemu-img convert -f qcow2 -O raw $< $@
%.raw: %.img
	qemu-img convert -f qcow2 -O raw $< $@


.PHONY: clean
clean:
	rm -rf images linux

