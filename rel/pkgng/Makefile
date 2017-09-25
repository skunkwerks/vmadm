INSTALL_DIR=usr/local/sbin
DEPS="openssl"
REBARPROFILE ?= default

include ../../config.mk
include pkgng.mk

.PHONY: prepare

prepare:
	mkdir -p $(STAGE_DIR)/$(INSTALL_DIR)
	mkdir -p $(STAGE_DIR)/usr/local/etc/vmadm
	mkdir -p $(STAGE_DIR)/var/imgadm/images
	cp -r ../../target/release/vmadm $(STAGE_DIR)/$(INSTALL_DIR)
