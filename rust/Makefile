# SPDX-License-Identifier: GPL-2.0

obj-$(CONFIG_RUST) += core.o compiler_builtins.o helpers.o
extra-$(CONFIG_RUST) += exports_core_generated.h

extra-$(CONFIG_RUST) += libmacros.so

extra-$(CONFIG_RUST) += bindings_generated.rs
obj-$(CONFIG_RUST) += alloc.o kernel.o
extra-$(CONFIG_RUST) += exports_alloc_generated.h exports_kernel_generated.h

ifdef CONFIG_RUST_BUILD_ASSERT_DENY
extra-$(CONFIG_RUST) += build_error.o
else
obj-$(CONFIG_RUST) += build_error.o
endif

obj-$(CONFIG_RUST) += exports.o

RUSTDOC = rustdoc

quiet_cmd_rustdoc = RUSTDOC $(if $(filter --test,$(rustdoc_target_flags)),T, ) $(if $(rustdoc_host),H, ) $<
      cmd_rustdoc = \
	RUST_BINDINGS_FILE=$(abspath $(objtree)/rust/bindings_generated.rs) \
	$(RUSTDOC) $(if $(rustdoc_host),,$(rustc_cross_flags)) \
		$(filter-out --emit=%, $(rustc_flags)) $(rustc_target_flags) $(rustdoc_target_flags) \
		-L $(objtree)/rust/$(if $(filter --test,$(rustdoc_target_flags)),test/) \
		--output $(objtree)/rust/doc --crate-name $(subst rusttest-,,$(subst rustdoc-,,$@)) \
		-Fmissing-docs @$(objtree)/include/generated/rustc_cfg $<

rustdoc: rustdoc-macros rustdoc-compiler_builtins rustdoc-kernel

rustdoc-macros: private rustdoc_host = yes
rustdoc-macros: private rustc_target_flags = --crate-type proc-macro \
    --extern proc_macro
rustdoc-macros: $(srctree)/rust/macros/lib.rs FORCE
	$(call if_changed,rustdoc)

rustdoc-compiler_builtins: $(srctree)/rust/compiler_builtins.rs FORCE
	$(call if_changed,rustdoc)

rustdoc-kernel: private rustc_target_flags = --extern alloc \
    --extern build_error \
    --extern macros=$(objtree)/rust/libmacros.so
rustdoc-kernel: $(srctree)/rust/kernel/lib.rs rustdoc-macros \
    $(objtree)/rust/libmacros.so $(objtree)/rust/bindings_generated.rs FORCE
	$(call if_changed,rustdoc)

quiet_cmd_rustc_test_library = RUSTC TL $<
      cmd_rustc_test_library = \
	RUST_BINDINGS_FILE=$(abspath $(objtree)/rust/bindings_generated.rs) \
	$(RUSTC) $(filter-out -Cpanic=abort, $(filter-out --emit=%, $(rustc_flags))) \
		$(rustc_target_flags) --crate-type $(if $(rustc_test_library_proc),proc-macro,rlib) \
		--out-dir $(objtree)/rust/test/ --cfg testlib \
		-L $(objtree)/rust/test/ --crate-name $(subst rusttest-,,$(subst rusttestlib-,,$@)) $<

rusttestlib-build_error: $(srctree)/rust/build_error.rs FORCE
	$(call if_changed,rustc_test_library)

rusttestlib-macros: private rustc_target_flags = --extern proc_macro
rusttestlib-macros: private rustc_test_library_proc = yes
rusttestlib-macros: $(srctree)/rust/macros/lib.rs FORCE
	$(call if_changed,rustc_test_library)

# We cannot use `-Zpanic-abort-tests` because some tests are dynamic,
# so for the moment we skip `-Cpanic=abort`.
quiet_cmd_rustc_test = RUSTC T  $<
      cmd_rustc_test = \
	RUST_BINDINGS_FILE=$(abspath $(objtree)/rust/bindings_generated.rs) \
	$(RUSTC) --test $(filter-out -Cpanic=abort, $(filter-out --emit=%, $(rustc_flags))) \
		$(rustc_target_flags) --out-dir $(objtree)/rust/test \
		-L $(objtree)/rust/test/ --crate-name $(subst rusttest-,,$@) $<; \
		$(objtree)/rust/test/$(subst rusttest-,,$@) $(rustc_test_run_flags)

rusttest: rusttest-macros rusttest-kernel

rusttest-macros: private rustc_target_flags = --extern proc_macro
rusttest-macros: private rustdoc_host = yes
rusttest-macros: private rustdoc_target_flags = --test --crate-type proc-macro
rusttest-macros: $(srctree)/rust/macros/lib.rs FORCE
	$(call if_changed,rustc_test)
	$(call if_changed,rustdoc)

rusttest-kernel: private rustc_target_flags = --extern alloc \
    --extern build_error \
    --extern macros=$(objtree)/rust/test/libmacros.so
rusttest-kernel: private rustc_test_run_flags = \
    --skip bindgen_test_layout_
rusttest-kernel: private rustdoc_host = yes
rusttest-kernel: private rustdoc_target_flags = --test
rusttest-kernel: $(srctree)/rust/kernel/lib.rs rusttestlib-build_error \
    rusttestlib-macros FORCE
	$(call if_changed,rustc_test)
	$(call if_changed,rustc_test_library)
	$(call if_changed,rustdoc)

ifdef CONFIG_CC_IS_CLANG
bindgen_c_flags = $(c_flags)
else
# bindgen relies on libclang to parse C. Ideally, bindgen would support a GCC
# plugin backend and/or the Clang driver would be perfectly compatible with GCC.
#
# For the moment, here we are tweaking the flags on the fly. Some config
# options may not work (e.g. `GCC_PLUGIN_RANDSTRUCT` if we end up using one
# of those structs). We might want to redo how Clang flags are kept track of
# in the general `Makefile` even for GCC builds, similar to what we did with
# `TENTATIVE_CLANG_FLAGS`.
bindgen_skip_c_flags := -mno-fp-ret-in-387 -mpreferred-stack-boundary=% \
	-mskip-rax-setup -mgeneral-regs-only -msign-return-address=% \
	-mindirect-branch=thunk-extern -mindirect-branch-register -mrecord-mcount \
	-mabi=lp64 -mstack-protector-guard% -fconserve-stack -falign-jumps=% \
	-falign-loops=% -fno-ipa-cp-clone -fno-partial-inlining \
	-fno-reorder-blocks -fno-allow-store-data-races -fasan-shadow-offset=% \
	-Wno-packed-not-aligned -Wno-format-truncation -Wno-format-overflow \
	-Wno-stringop-truncation -Wno-unused-but-set-variable \
	-Wno-stringop-overflow -Wno-restrict -Wno-maybe-uninitialized \
	-Werror=designated-init -Wno-zero-length-bounds \
	--param=% --param asan-%

# PowerPC
bindgen_skip_c_flags += -mtraceback=no -mno-pointers-to-nested-functions \
	-mno-string -mno-strict-align

bindgen_extra_c_flags = $(TENTATIVE_CLANG_FLAGS) -Wno-address-of-packed-member
bindgen_c_flags = $(filter-out $(bindgen_skip_c_flags), $(c_flags)) \
	$(bindgen_extra_c_flags)
endif

# To avoid several recompilations in PowerPC, which inserts `-D_TASK_CPU`
bindgen_c_flags_final = $(filter-out -D_TASK_CPU=%, $(bindgen_c_flags))

quiet_cmd_bindgen = BINDGEN $@
      cmd_bindgen = \
	$(BINDGEN) $< $(shell grep -v '^\#\|^$$' $(srctree)/rust/bindgen_parameters) \
		--use-core --with-derive-default --ctypes-prefix c_types \
		--no-debug '.*' \
		--size_t-is-usize -o $@ -- $(bindgen_c_flags_final) -DMODULE

$(objtree)/rust/bindings_generated.rs: $(srctree)/rust/kernel/bindings_helper.h \
	$(srctree)/rust/bindgen_parameters FORCE
	$(call if_changed_dep,bindgen)

quiet_cmd_exports = EXPORTS $@
      cmd_exports = \
	$(NM) -p --defined-only $< \
		| grep -E ' (T|R|D) ' | cut -d ' ' -f 3 \
		| xargs -Isymbol \
		echo 'EXPORT_SYMBOL_RUST_GPL(symbol);' > $@

$(objtree)/rust/exports_core_generated.h: $(objtree)/rust/core.o FORCE
	$(call if_changed,exports)

$(objtree)/rust/exports_alloc_generated.h: $(objtree)/rust/alloc.o FORCE
	$(call if_changed,exports)

$(objtree)/rust/exports_kernel_generated.h: $(objtree)/rust/kernel.o FORCE
	$(call if_changed,exports)

# `-Cpanic=unwind -Cforce-unwind-tables=y` overrides `rustc_flags` in order to
# avoid the https://github.com/rust-lang/rust/issues/82320 rustc crash.
quiet_cmd_rustc_procmacro = $(RUSTC_OR_CLIPPY_QUIET) P $@
      cmd_rustc_procmacro = \
	$(RUSTC_OR_CLIPPY) $(rustc_flags) \
		--emit=dep-info,link --extern proc_macro \
		-Cpanic=unwind -Cforce-unwind-tables=y \
		--crate-type proc-macro --out-dir $(objtree)/rust/ \
		--crate-name $(patsubst lib%.so,%,$(notdir $@)) $<; \
	mv $(objtree)/rust/$(patsubst lib%.so,%,$(notdir $@)).d $(depfile); \
	sed -i '/^\#/d' $(depfile)

# Procedural macros can only be used with the `rustc` that compiled it.
# Therefore, to get `libmacros.so` automatically recompiled when the compiler
# version changes, we add `core.o` as a dependency (even if it is not needed).
$(objtree)/rust/libmacros.so: $(srctree)/rust/macros/lib.rs \
	$(objtree)/rust/core.o FORCE
	$(call if_changed_dep,rustc_procmacro)

quiet_cmd_rustc_library = $(if $(skip_clippy),RUSTC,$(RUSTC_OR_CLIPPY_QUIET)) L $@
      cmd_rustc_library = \
	RUST_BINDINGS_FILE=$(abspath $(objtree)/rust/bindings_generated.rs) \
	$(if $(skip_clippy),$(RUSTC),$(RUSTC_OR_CLIPPY)) \
		$(rustc_flags) $(rustc_cross_flags) $(rustc_target_flags) \
		--crate-type rlib --out-dir $(objtree)/rust/ -L $(objtree)/rust/ \
		--crate-name $(patsubst %.o,%,$(notdir $@)) $<; \
	mv $(objtree)/rust/$(patsubst %.o,%,$(notdir $@)).d $(depfile); \
	sed -i '/^\#/d' $(depfile) \
	$(if $(rustc_objcopy),;$(OBJCOPY) $(rustc_objcopy) $@)

# `$(rustc_flags)` is passed in case the user added `--sysroot`.
rustc_sysroot = $(shell $(RUSTC) $(rustc_flags) --print sysroot)
RUST_LIB_SRC ?= $(rustc_sysroot)/lib/rustlib/src/rust/library

rust-analyzer:
	$(Q)$(srctree)/scripts/generate_rust_analyzer.py $(srctree) $(objtree) $(RUST_LIB_SRC) $(objtree)/rust/bindings_generated.rs > $(objtree)/rust-project.json

$(objtree)/rust/compiler_builtins.o: private rustc_objcopy = -w -W '__*'
$(objtree)/rust/compiler_builtins.o: $(srctree)/rust/compiler_builtins.rs \
    $(objtree)/rust/core.o FORCE
	$(call if_changed_dep,rustc_library)

$(objtree)/rust/build_error.o: $(srctree)/rust/build_error.rs \
    $(objtree)/rust/compiler_builtins.o FORCE
	$(call if_changed_dep,rustc_library)

# ICE on `--extern macros`: https://github.com/rust-lang/rust/issues/56935
$(objtree)/rust/kernel.o: private rustc_target_flags = --extern alloc \
    --extern build_error \
    --extern macros=$(objtree)/rust/libmacros.so
$(objtree)/rust/kernel.o: $(srctree)/rust/kernel/lib.rs $(objtree)/rust/alloc.o \
    $(objtree)/rust/build_error.o \
    $(objtree)/rust/libmacros.so $(objtree)/rust/bindings_generated.rs FORCE
	$(call if_changed_dep,rustc_library)

# Targets that need to expand twice
.SECONDEXPANSION:
$(objtree)/rust/core.o: private skip_clippy = 1
$(objtree)/rust/core.o: $$(RUST_LIB_SRC)/core/src/lib.rs FORCE
	$(call if_changed_dep,rustc_library)

$(objtree)/rust/alloc.o: private skip_clippy = 1
$(objtree)/rust/alloc.o: $$(RUST_LIB_SRC)/alloc/src/lib.rs \
    $(objtree)/rust/compiler_builtins.o FORCE
	$(call if_changed_dep,rustc_library)
