MAKEFILE_DIR := $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))

ifndef BUILD
BUILD := $(MAKEFILE_DIR)/+build
endif

ifndef O
O := $(MAKEFILE_DIR)/+install
endif

.PHONY: all initramfs install-tar build install
all: build initramfs install-tar

initramfs: $(BUILD)/core.gz
install-tar: $(BUILD)/install.tar.gz

# Taskset builder is currently under costruction
# build: cgroup periodic scripts tasksets
build: cgroup periodic rt-app scripts tasksets

install: build
	mkdir -p $(O)
	cp -ur $(BUILD)/mnt/root/* $(O)/

# test software
.PHONY: cgroup
cgroup: $(BUILD)/cgroup.keep

RUSTFLAGS="-C target-feature=+crt-static"
$(BUILD)/cgroup.keep: $(BUILD)/mnt/.keep $(BUILD)/.keep
	mkdir -p $(BUILD)/test_suite
	RUSTFLAGS=$(RUSTFLAGS) \
		cargo build --release --target x86_64-unknown-linux-gnu \
		--target-dir "$(BUILD)/test_suite"

	mkdir -p $(BUILD)/mnt/root/test_suite
	find $(BUILD)/test_suite/x86_64-unknown-linux-gnu/release/ \
		-maxdepth 1 -executable -type f | \
		xargs -I{} cp -u "{}" $(BUILD)/mnt/root/test_suite/

# tasksets
.PHONY: tasksets
tasksets: $(BUILD)/mnt/root/tasksets/.keep

.PRECIOUS: $(BUILD)/mnt/root/tasksets/.keep
$(BUILD)/mnt/root/tasksets/.keep: $(BUILD)/.keep
	cargo run --bin taskset_gen --release \
	--target-dir "$(BUILD)/test_suite" -- -O $(BUILD)/mnt/root/tasksets
	touch $@

# extra scripts
SCRIPTS = $(wildcard scripts/*)

.PHONY: scripts
scripts: $(BUILD)/scripts.keep

$(BUILD)/scripts.keep: $(SCRIPTS)
	cp -ur scripts/* $(BUILD)/mnt/root
	touch $@

# periodic task runner
.PHONY: periodic
periodic: $(BUILD)/mnt/root/bin/periodic_task $(BUILD)/mnt/root/bin/periodic_thread

$(BUILD)/PeriodicTask/.keep: $(BUILD)/.keep
	git init $(@D)
	git -C $(@D) fetch --depth=1 \
		https://gitlab.retis.santannapisa.it/l.abeni/PeriodicTask.git \
		8b1839d2c2207cbb7e80f25e9d6773bbeab6630e
	git -C $(@D) checkout FETCH_HEAD
	sed -i '18 c#define MAX_TH 50' $(@D)/periodic_thread.c
	touch $@

$(BUILD)/mnt/root/bin/periodic_task: $(BUILD)/PeriodicTask/.keep
	make -C $(<D) periodic_task
	mkdir -p $(@D)
	cp -u $(<D)/periodic_task $@

$(BUILD)/mnt/root/bin/periodic_thread: $(BUILD)/PeriodicTask/.keep
	make -C $(<D) periodic_thread
	mkdir -p $(@D)
	cp -u $(<D)/periodic_thread $@

# rt-app task runner
.PHONY: rt-app
rt-app: $(BUILD)/mnt/root/bin/rt-app

$(BUILD)/rt-app/json-c/.keep: $(BUILD)/.keep
	git init $(@D)
	git -C $(@D) fetch --depth=1 \
		https://github.com/json-c/json-c
	git -C $(@D) checkout FETCH_HEAD
	cd $(@D); cmake .; make
	touch $@

$(BUILD)/rt-app/rt-app/.keep: $(BUILD)/.keep $(BUILD)/rt-app/json-c/.keep
	git init $(@D)
	git -C $(@D) fetch --depth=1 \
		http://github.com/scheduler-tools/rt-app
	git -C $(@D) checkout FETCH_HEAD
	cd $(@D); \
		export ac_cv_lib_json_c_json_object_from_file=yes; \
    	export ac_cv_lib_numa_numa_available=no; \
		./autogen.sh; \
    	./configure --host=amd64-linux-gnu \
			LDFLAGS="-L$(BUILD)/rt-app/json-c" \
			CFLAGS="-I$(BUILD)/rt-app/"; \
    	AM_LDFLAGS="-all-static" make
	touch $@

$(BUILD)/mnt/root/bin/rt-app: $(BUILD)/rt-app/rt-app/.keep
	mkdir -p $(@D)
	cp $(BUILD)/rt-app/rt-app/src/rt-app $@

# busybox (only for initramfs)
.PHONY: busybox
busybox: $(BUILD)/initrd-busybox.gz

# get busybox builder and update the config
$(BUILD)/BuildCore/.keep: $(BUILD)/.keep
	git init $(@D)
	git -C $(@D) fetch --depth=1 \
		https://gitlab.retis.santannapisa.it/l.abeni/BuildCore.git \
		715962453dc89fb694f1193278d9f45304f03741
	git -C $(@D) checkout FETCH_HEAD
	sed -i '967 cCONFIG_TC=n' $(@D)/Configs/config-busybox-3
	sed -i '11 cSUDOVER=1.9.17p2' $(@D)/buildcore.sh
	touch $@

$(BUILD)/initrd-busybox.gz: $(BUILD)/BuildCore/.keep
	mkdir -p $(BUILD)/busybox
	sh $(MAKEFILE_DIR)/utils/ubuntu.sh "sh ../BuildCore/buildcore.sh ./$(@F)"
	cp -u $(BUILD)/busybox/bb_build-1.36.1/_install/initrd-busybox.gz $@

### compressed targets
# initramfs
$(BUILD)/core.gz: $(BUILD)/initrd-busybox.gz $(BUILD)/initrd.gz
	rm -f $@
	touch $@
	cat $(BUILD)/initrd.gz >> $@
	cat $(BUILD)/initrd-busybox.gz >> $@

$(BUILD)/initrd.gz: build
	cd $(BUILD)/mnt/ && find . | cpio -o -H newc | gzip > ../initrd.gz

# tar compressed archive
$(BUILD)/install.tar.gz: build
	cd $(BUILD)/mnt/root && tar -czvf ../../install.tar.gz .

# generic
$(BUILD)/mnt/.keep:
	mkdir -p $(@D)
	touch $@

$(BUILD)/.keep:
	mkdir -p $(@D)
	touch $@

.PHONY: clean
clean:
	rm -rf $(BUILD)
