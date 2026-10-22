// NVIDIA driver installation, ported from Soplos Welcome 3.0 (commands/drivers.rs).
// The scripts are the same ones Welcome runs, but here they are not written to
// a file and run from there: the helper that runs as root builds them itself
// (see privileged.rs) from a branch name checked against a closed list, so no
// script text ever comes from outside the binary.

/// Driver packages of the Debian repositories: the 550 branch and the legacy
/// 470, 390 and 340 ones (these need Debian Sid).
pub const REPO_PACKAGES: [&str; 4] = ["nvidia-driver", "nvidia-tesla-470-driver", "nvidia-legacy-390xx-driver", "nvidia-legacy-340xx-driver"];

/// Branches of NVIDIA's own CUDA repository.
pub const CUDA_BRANCHES: [&str; 3] = ["580", "590", "610"];

// === NVIDIA DKMS compatibility patches for Soplos kernel 7.x ===
// Ported verbatim from drivers_tab.py (NV_MMAP_VMA_LOCK_PATCH / NV_STRING_H_PATCH /
// get_nvidia_dkms_patch_commands / LLVM_DKMS_TOOLCHAIN). Without these, DKMS
// fails to build the NVIDIA module against this kernel and, since the driver
// package blacklists nouveau, the machine boots with no graphics driver at all.

const NV_MMAP_VMA_LOCK_PATCH: &str = r#"--- a/nvidia/nv-mmap.c
+++ b/nvidia/nv-mmap.c
@@ -868,17 +868,24 @@

     nvl->safe_to_mmap = safe_to_mmap;
 }
+#ifndef VM_REFCNT_EXCLUDE_READERS_FLAG
+#define VM_REFCNT_EXCLUDE_READERS_FLAG VMA_LOCK_OFFSET
+#else
+#define NV_VMA_WRITE_LOCKED_ONE_ARG 1
+#endif
+
+

 #if !NV_CAN_CALL_VMA_START_WRITE
 static NvBool nv_vma_enter_locked(struct vm_area_struct *vma, NvBool detaching)
 {
-    NvU32 tgt_refcnt = VMA_LOCK_OFFSET;
+    NvU32 tgt_refcnt = VM_REFCNT_EXCLUDE_READERS_FLAG;
     NvBool interrupted = NV_FALSE;
     if (!detaching)
     {
         tgt_refcnt++;
     }
-    if (!refcount_add_not_zero(VMA_LOCK_OFFSET, &vma->vm_refcnt))
+    if (!refcount_add_not_zero(VM_REFCNT_EXCLUDE_READERS_FLAG, &vma->vm_refcnt))
     {
         return NV_FALSE;
     }
@@ -908,7 +915,7 @@
     if (interrupted)
     {
         // Clean up on error: release refcount and dep_map
-        refcount_sub_and_test(VMA_LOCK_OFFSET, &vma->vm_refcnt);
+        refcount_sub_and_test(VM_REFCNT_EXCLUDE_READERS_FLAG, &vma->vm_refcnt);
         rwsem_release(&vma->vmlock_dep_map, _RET_IP_);
         return NV_FALSE;
     }
@@ -924,7 +931,11 @@
 {
     NvU32 mm_lock_seq;
     NvBool locked;
+#ifdef NV_VMA_WRITE_LOCKED_ONE_ARG
+    if (__is_vma_write_locked(vma))
+#else
     if (__is_vma_write_locked(vma, &mm_lock_seq))
+#endif
         return;

     locked = nv_vma_enter_locked(vma, NV_FALSE);
@@ -933,7 +944,7 @@
     if (locked)
     {
         NvBool detached;
-        detached = refcount_sub_and_test(VMA_LOCK_OFFSET, &vma->vm_refcnt);
+        detached = refcount_sub_and_test(VM_REFCNT_EXCLUDE_READERS_FLAG, &vma->vm_refcnt);
         rwsem_release(&vma->vmlock_dep_map, _RET_IP_);
         WARN_ON_ONCE(detached);
     }
"#;

const NV_STRING_H_PATCH: &str = r#"--- a/nvidia/os-interface.c
+++ b/nvidia/os-interface.c
@@ -35,6 +35,7 @@
 #include <linux/mmzone.h>
 #include <linux/numa.h>
 #include <linux/cpuset.h>
 #include <linux/sys_soc.h>
+#include <linux/string.h>

 extern char *NVreg_TemporaryFilePath;
"#;

// Fixes dmem_cgroup_register_region() API change on Linux 7.3+:
// kernel commit e11640b9cea4 ("cgroup/dmem: Introduce struct dmem_cgroup_init
// for region initialization") changed the first argument from a bare u64 size
// to a const struct dmem_cgroup_init *. NVIDIA 610 (open) passes size
// directly, which GCC rejects with -Wint-conversion on kernel 7.3+.
const NV_DMEM_CGROUP_INIT_PATCH: &str = r#"--- a/nvidia/os-interface.c
+++ b/nvidia/os-interface.c
@@ -2818,6 +2818,15 @@
 void* NV_API_CALL os_dmem_cgroup_register_region(NvU64 size, const char *name)
 {
+#if LINUX_VERSION_CODE >= KERNEL_VERSION(7, 3, 0)
+    struct dmem_cgroup_init init = {
+        .size = size,
+        .ops  = NULL,
+        .reclaim_priv = NULL,
+    };
+    void *region = dmem_cgroup_register_region(&init, name);
+#else
     void *region = dmem_cgroup_register_region(size, name);
+#endif
     if (IS_ERR(region))
     {
         return NULL;
"#;

// NVIDIA 550 (Debian nvidia-kernel-dkms, 550.163.01) on kernels 7.2+: linux/of_gpio.h
// was removed in 7.1, strncpy() in 7.2, and struct drm_atomic_state became
// struct drm_atomic_commit in 7.2. The patch adds the of_gpio.h compat block that
// NVIDIA 610 already carries, replaces the five strncpy() calls, maps the renamed
// DRM names and forces five nvidia-drm configure-time probes. Same patch as
// nvidia-patches/patches/nvidia-550-kernel7.2-compat.patch.
const NV_550_KERNEL72_COMPAT_PATCH: &str = r#"--- a/common/inc/nv-linux.h
+++ b/common/inc/nv-linux.h
@@ -2038,7 +2038,71 @@
 #endif
 
 #if defined(NV_LINUX_OF_GPIO_H_PRESENT)
+#if __has_include(<linux/of_gpio.h>)
 #include <linux/of_gpio.h>
+#else
+#include <linux/gpio/driver.h>
+
+static inline int __to_hwgpio(const struct gpio_device *gdev,
+                              const struct of_phandle_args *gpiospec)
+{
+#if defined(CONFIG_OF_GPIO)
+    /*
+     * Use the chip's of_xlate callback to translate the DT GPIO
+     * specifier into a linear offset.  Tegra GPIO controllers encode
+     * port and pin in args[0] and of_xlate sums per-port pin counts
+     * to produce the real offset.
+     */
+    struct gpio_chip *chip = gpio_device_get_chip(gdev);
+
+    if (chip->of_xlate)
+        return chip->of_xlate(chip, gpiospec, NULL);
+#endif
+
+    return gpiospec->args[0];
+}
+
+/*
+ * of_get_named_gpio() was removed along with linux/of_gpio.h by commit
+ * 51aaf65bbd21 ("gpio: of: Remove <linux/of_gpio.h>"). Provide a compat
+ * implementation using the remaining public GPIO APIs.
+ */
+static inline int of_get_named_gpio(const struct device_node *np,
+                                    const char *propname, int index)
+{
+    struct of_phandle_args gpiospec;
+    struct gpio_device *gdev;
+    struct gpio_desc *desc;
+    int ret, hwgpio;
+
+    if (!np)
+        return -ENOENT;
+
+    ret =
+        of_parse_phandle_with_args_map(np, propname, "gpio", index, &gpiospec);
+    if (ret)
+        return ret;
+
+    gdev = gpio_device_find_by_fwnode(of_fwnode_handle(gpiospec.np));
+    of_node_put(gpiospec.np);
+    if (!gdev)
+        return -EPROBE_DEFER;
+
+    hwgpio = __to_hwgpio(gdev, &gpiospec);
+    if (hwgpio < 0) {
+        gpio_device_put(gdev);
+        return hwgpio;
+    }
+
+    desc = gpio_device_get_desc(gdev, hwgpio);
+    gpio_device_put(gdev);
+
+    if (IS_ERR(desc))
+        return PTR_ERR(desc);
+
+    return desc_to_gpio(desc);
+}
+#endif
 #endif
 
 #if defined(NV_LINUX_OF_DEVICE_H_PRESENT)
--- a/nvidia/os-interface.c
+++ b/nvidia/os-interface.c
@@ -750,8 +750,9 @@
 void NV_API_CALL os_get_current_process_name(char *buf, NvU32 len)
 {
     task_lock(current);
-    strncpy(buf, current->comm, len - 1);
-    buf[len - 1] = '\0';
+    if (strscpy(buf, current->comm, len) < 0) {
+        // strscpy() returns -E2BIG on truncation, which is acceptable here.
+    }
     task_unlock(current);
 }
 
--- a/nvidia/linux_nvswitch.c
+++ b/nvidia/linux_nvswitch.c
@@ -2026,7 +2026,7 @@
         return -NVL_ERR_GENERIC;
     }
 
-    strncpy(regkey_val, regkey_val_start, regkey_val_len);
+    memcpy(regkey_val, regkey_val_start, regkey_val_len);
     regkey_val[regkey_val_len] = '\0';
 
     if (nvswitch_os_strtouint(regkey_val, data) != 0)
@@ -2381,7 +2381,13 @@
     NvLength length
 )
 {
-    return strncpy(dest, src, length);
+    NvLength i;
+
+    for (i = 0; i < length && src[i] != '\0'; i++)
+        dest[i] = src[i];
+    for (; i < length; i++)
+        dest[i] = '\0';
+    return dest;
 }
 
 int
--- a/nvidia-modeset/nvidia-modeset-linux.c
+++ b/nvidia-modeset/nvidia-modeset-linux.c
@@ -351,7 +351,13 @@
 
 char* nvkms_strncpy(char *dest, const char *src, size_t n)
 {
-    return strncpy(dest, src, n);
+    size_t i;
+
+    for (i = 0; i < n && src[i] != '\0'; i++)
+        dest[i] = src[i];
+    for (; i < n; i++)
+        dest[i] = '\0';
+    return dest;
 }
 
 void nvkms_usleep(NvU64 usec)
--- a/nvidia-uvm/uvm_pmm_gpu.c
+++ b/nvidia-uvm/uvm_pmm_gpu.c
@@ -3038,7 +3038,7 @@
             size_t size;
             size_t align;
             if (level == 0) {
-                strncpy(chunk_split_cache[level].name, "uvm_gpu_chunk_t", sizeof(chunk_split_cache[level].name) - 1);
+                strscpy(chunk_split_cache[level].name, "uvm_gpu_chunk_t", sizeof(chunk_split_cache[level].name));
                 size = sizeof(uvm_gpu_chunk_t);
                 align = __alignof__(uvm_gpu_chunk_t);
             } else {
--- a/nvidia-drm/nvidia-drm-conftest.h
+++ b/nvidia-drm/nvidia-drm-conftest.h
@@ -215,4 +215,41 @@
 
 #endif /* defined(NV_BSD) */
 
+/*
+ * Linux 7.2 renamed struct drm_atomic_state to struct drm_atomic_commit and
+ * the helpers that operate on it. Map the old names onto the new ones.
+ */
+#include <linux/version.h>
+#if LINUX_VERSION_CODE >= KERNEL_VERSION(7, 2, 0)
+#define drm_atomic_state                        drm_atomic_commit
+#define drm_atomic_state_alloc                  drm_atomic_commit_alloc
+#define drm_atomic_state_get                    drm_atomic_commit_get
+#define drm_atomic_state_put                    drm_atomic_commit_put
+#define drm_atomic_state_free                   drm_atomic_commit_put
+#define drm_atomic_state_clear                  drm_atomic_commit_clear
+#define drm_atomic_state_default_clear          drm_atomic_commit_default_clear
+#define drm_atomic_state_default_release        drm_atomic_commit_default_release
+#define drm_atomic_state_init                   drm_atomic_commit_init
+
+/*
+ * The configure-time probes below look for struct drm_atomic_state, which no
+ * longer exists, so they report the pre-7.2 behavior. Force the 7.2 behavior.
+ */
+#if !defined(NV_DRM_PLANE_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG)
+#define NV_DRM_PLANE_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG
+#endif
+#if !defined(NV_DRM_CRTC_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG)
+#define NV_DRM_CRTC_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG
+#endif
+#if !defined(NV_DRM_ATOMIC_STATE_REF_COUNTING_PRESENT)
+#define NV_DRM_ATOMIC_STATE_REF_COUNTING_PRESENT
+#endif
+#if !defined(NV_DRM_ATOMIC_HELPER_SWAP_STATE_HAS_STALL_ARG)
+#define NV_DRM_ATOMIC_HELPER_SWAP_STATE_HAS_STALL_ARG
+#endif
+#if !defined(NV_DRM_ATOMIC_HELPER_SWAP_STATE_RETURN_INT)
+#define NV_DRM_ATOMIC_HELPER_SWAP_STATE_RETURN_INT
+#endif
+#endif
+
 #endif /* defined(__NVIDIA_DRM_CONFTEST_H__) */
"#;

// NVIDIA 590 open modules (590.48.01, nvidia-open-590) on kernels 7.2+: the same
// of_gpio.h / strncpy() / DRM atomic commit fixes as the 550 patch above, plus the
// nvidia-uvm changes NVIDIA 610 already carries (dev_pagemap_ops page_free became
// folio_free, and zone_device_page_init() takes the pgmap and an order), backported
// through two conftest.sh probes. Applied from kernel-open/ after the VMA patch.
// Same patch as nvidia-patches/patches/nvidia-590-kernel7.2-compat.patch.
const NV_590_KERNEL72_COMPAT_PATCH: &str = r#"--- a/common/inc/nv-linux.h
+++ b/common/inc/nv-linux.h
@@ -1714,7 +1714,71 @@
 #include <linux/reset.h>
 #include <linux/dma-buf.h>
 #include <linux/gpio.h>
+#if __has_include(<linux/of_gpio.h>)
 #include <linux/of_gpio.h>
+#else
+#include <linux/gpio/driver.h>
+
+static inline int __to_hwgpio(const struct gpio_device *gdev,
+                              const struct of_phandle_args *gpiospec)
+{
+#if defined(CONFIG_OF_GPIO)
+    /*
+     * Use the chip's of_xlate callback to translate the DT GPIO
+     * specifier into a linear offset.  Tegra GPIO controllers encode
+     * port and pin in args[0] and of_xlate sums per-port pin counts
+     * to produce the real offset.
+     */
+    struct gpio_chip *chip = gpio_device_get_chip(gdev);
+
+    if (chip->of_xlate)
+        return chip->of_xlate(chip, gpiospec, NULL);
+#endif
+
+    return gpiospec->args[0];
+}
+
+/*
+ * of_get_named_gpio() was removed along with linux/of_gpio.h by commit
+ * 51aaf65bbd21 ("gpio: of: Remove <linux/of_gpio.h>"). Provide a compat
+ * implementation using the remaining public GPIO APIs.
+ */
+static inline int of_get_named_gpio(const struct device_node *np,
+                                    const char *propname, int index)
+{
+    struct of_phandle_args gpiospec;
+    struct gpio_device *gdev;
+    struct gpio_desc *desc;
+    int ret, hwgpio;
+
+    if (!np)
+        return -ENOENT;
+
+    ret =
+        of_parse_phandle_with_args_map(np, propname, "gpio", index, &gpiospec);
+    if (ret)
+        return ret;
+
+    gdev = gpio_device_find_by_fwnode(of_fwnode_handle(gpiospec.np));
+    of_node_put(gpiospec.np);
+    if (!gdev)
+        return -EPROBE_DEFER;
+
+    hwgpio = __to_hwgpio(gdev, &gpiospec);
+    if (hwgpio < 0) {
+        gpio_device_put(gdev);
+        return hwgpio;
+    }
+
+    desc = gpio_device_get_desc(gdev, hwgpio);
+    gpio_device_put(gdev);
+
+    if (IS_ERR(desc))
+        return PTR_ERR(desc);
+
+    return desc_to_gpio(desc);
+}
+#endif
 #include <linux/of_device.h>
 #include <linux/of_platform.h>
 
--- a/nvidia/os-interface.c
+++ b/nvidia/os-interface.c
@@ -759,8 +759,9 @@
 void NV_API_CALL os_get_current_process_name(char *buf, NvU32 len)
 {
     task_lock(current);
-    strncpy(buf, current->comm, len - 1);
-    buf[len - 1] = '\0';
+    if (strscpy(buf, current->comm, len) < 0) {
+        // strscpy() returns -E2BIG on truncation, which is acceptable here.
+    }
     task_unlock(current);
 }
 
--- a/nvidia/linux_nvswitch.c
+++ b/nvidia/linux_nvswitch.c
@@ -2059,7 +2059,7 @@
         return -NVL_ERR_GENERIC;
     }
 
-    strncpy(regkey_val, regkey_val_start, regkey_val_len);
+    memcpy(regkey_val, regkey_val_start, regkey_val_len);
     regkey_val[regkey_val_len] = '\0';
 
     if (nvswitch_os_strtouint(regkey_val, data) != 0)
@@ -2414,7 +2414,13 @@
     NvLength length
 )
 {
-    return strncpy(dest, src, length);
+    NvLength i;
+
+    for (i = 0; i < length && src[i] != '\0'; i++)
+        dest[i] = src[i];
+    for (; i < length; i++)
+        dest[i] = '\0';
+    return dest;
 }
 
 int
--- a/nvidia-modeset/nvidia-modeset-linux.c
+++ b/nvidia-modeset/nvidia-modeset-linux.c
@@ -647,7 +647,13 @@
 
 char* nvkms_strncpy(char *dest, const char *src, size_t n)
 {
-    return strncpy(dest, src, n);
+    size_t i;
+
+    for (i = 0; i < n && src[i] != '\0'; i++)
+        dest[i] = src[i];
+    for (; i < n; i++)
+        dest[i] = '\0';
+    return dest;
 }
 
 void nvkms_usleep(NvU64 usec)
--- a/nvidia-uvm/uvm_pmm_gpu.c
+++ b/nvidia-uvm/uvm_pmm_gpu.c
@@ -2786,7 +2786,7 @@
             size_t size;
             size_t align;
             if (level == 0) {
-                strncpy(chunk_split_cache[level].name, "uvm_gpu_chunk_t", sizeof(chunk_split_cache[level].name) - 1);
+                strscpy(chunk_split_cache[level].name, "uvm_gpu_chunk_t", sizeof(chunk_split_cache[level].name));
                 size = sizeof(uvm_gpu_chunk_t);
                 align = __alignof__(uvm_gpu_chunk_t);
             } else {
@@ -3058,9 +3058,20 @@
     UVM_ENTRY_RET(devmem_fault(vmf));
 }
 
+#if defined(NV_PAGEMAP_OPS_HAS_FOLIO_FREE)
+static void devmem_folio_free(struct folio *folio)
+{
+    devmem_page_free(&folio->page);
+}
+#endif
+
 static const struct dev_pagemap_ops uvm_pmm_devmem_ops =
 {
+#if defined(NV_PAGEMAP_OPS_HAS_FOLIO_FREE)
+    .folio_free = devmem_folio_free,
+#else
     .page_free = devmem_page_free,
+#endif
     .migrate_to_ram = devmem_fault_entry,
 };
 
@@ -3163,9 +3174,20 @@
     device_p2p_page_free(page);
 }
 
+#if defined(NV_PAGEMAP_OPS_HAS_FOLIO_FREE)
+static void device_coherent_folio_free(struct folio *folio)
+{
+    device_coherent_page_free(&folio->page);
+}
+#endif
+
 static const struct dev_pagemap_ops uvm_device_coherent_pgmap_ops =
 {
+#if defined(NV_PAGEMAP_OPS_HAS_FOLIO_FREE)
+    .folio_free = device_coherent_folio_free,
+#else
     .page_free = device_coherent_page_free,
+#endif
 };
 
 static NV_STATUS uvm_pmm_cdmm_init(uvm_parent_gpu_t *parent_gpu)
@@ -3300,9 +3322,20 @@
 // ARM
 #if defined(CONFIG_PCI_P2PDMA) && defined(NV_STRUCT_PAGE_HAS_ZONE_DEVICE_DATA) && !defined(NVCPU_AARCH64)
 
+#if defined(NV_PAGEMAP_OPS_HAS_FOLIO_FREE)
+static void device_p2p_folio_free(struct folio *folio)
+{
+    device_p2p_page_free(&folio->page);
+}
+#endif
+
 static const struct dev_pagemap_ops uvm_device_p2p_pgmap_ops =
 {
+#if defined(NV_PAGEMAP_OPS_HAS_FOLIO_FREE)
+    .folio_free = device_p2p_folio_free,
+#else
     .page_free = device_p2p_page_free,
+#endif
 };
 
 void uvm_pmm_gpu_device_p2p_init(uvm_parent_gpu_t *parent_gpu)
--- a/nvidia-uvm/uvm_hmm.c
+++ b/nvidia-uvm/uvm_hmm.c
@@ -74,6 +74,19 @@
 #include "uvm_va_policy.h"
 #include "uvm_tools.h"
 
+//
+// Pass 0 as the order, when actual large order support is added this
+// function will need to be revisited
+//
+static __always_inline void nv_zone_device_page_init(struct page *page)
+{
+#if defined(NV_ZONE_DEVICE_PAGE_INIT_HAS_PGMAP_AND_ORDER_ARGS)
+    zone_device_page_init(page, page_pgmap(page), 0);
+#else
+    zone_device_page_init(page);
+#endif
+}
+
 // The function nv_PageSwapCache() wraps the check for page swap cache flag in
 // order to support a wide variety of kernel versions.
 // The function PageSwapCache() is removed after 32f51ead3d77 ("mm: remove
@@ -2140,7 +2153,7 @@
 
         UVM_ASSERT(!page_count(dpage));
         UVM_ASSERT(!dpage->zone_device_data);
-        zone_device_page_init(dpage);
+        nv_zone_device_page_init(dpage);
         dpage->zone_device_data = gpu_chunk;
         atomic64_inc(&va_block->hmm.va_space->hmm.allocated_page_count);
     }
--- a/nvidia-uvm/nvidia-uvm.Kbuild
+++ b/nvidia-uvm/nvidia-uvm.Kbuild
@@ -68,6 +68,8 @@
 NV_CONFTEST_TYPE_COMPILE_TESTS += mmu_interval_notifier
 NV_CONFTEST_TYPE_COMPILE_TESTS += sg_dma_page_iter
 NV_CONFTEST_TYPE_COMPILE_TESTS += struct_page_has_zone_device_data
+NV_CONFTEST_TYPE_COMPILE_TESTS += dev_pagemap_ops_has_folio_free
+NV_CONFTEST_TYPE_COMPILE_TESTS += zone_device_page_init_has_pgmap_and_order_args
 NV_CONFTEST_TYPE_COMPILE_TESTS += memory_device_coherent_present
 
 NV_CONFTEST_SYMBOL_COMPILE_TESTS += is_export_symbol_present_int_active_memcg
--- a/nvidia-drm/nvidia-drm-conftest.h
+++ b/nvidia-drm/nvidia-drm-conftest.h
@@ -207,4 +207,41 @@
 
 #endif /* defined(NV_BSD) */
 
+/*
+ * Linux 7.2 renamed struct drm_atomic_state to struct drm_atomic_commit and
+ * the helpers that operate on it. Map the old names onto the new ones.
+ */
+#include <linux/version.h>
+#if LINUX_VERSION_CODE >= KERNEL_VERSION(7, 2, 0)
+#define drm_atomic_state                        drm_atomic_commit
+#define drm_atomic_state_alloc                  drm_atomic_commit_alloc
+#define drm_atomic_state_get                    drm_atomic_commit_get
+#define drm_atomic_state_put                    drm_atomic_commit_put
+#define drm_atomic_state_free                   drm_atomic_commit_put
+#define drm_atomic_state_clear                  drm_atomic_commit_clear
+#define drm_atomic_state_default_clear          drm_atomic_commit_default_clear
+#define drm_atomic_state_default_release        drm_atomic_commit_default_release
+#define drm_atomic_state_init                   drm_atomic_commit_init
+
+/*
+ * The configure-time probes below look for struct drm_atomic_state, which no
+ * longer exists, so they report the pre-7.2 behavior. Force the 7.2 behavior.
+ */
+#if !defined(NV_DRM_PLANE_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG)
+#define NV_DRM_PLANE_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG
+#endif
+#if !defined(NV_DRM_CRTC_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG)
+#define NV_DRM_CRTC_ATOMIC_CHECK_HAS_ATOMIC_STATE_ARG
+#endif
+#if !defined(NV_DRM_ATOMIC_STATE_REF_COUNTING_PRESENT)
+#define NV_DRM_ATOMIC_STATE_REF_COUNTING_PRESENT
+#endif
+#if !defined(NV_DRM_ATOMIC_HELPER_SWAP_STATE_HAS_STALL_ARG)
+#define NV_DRM_ATOMIC_HELPER_SWAP_STATE_HAS_STALL_ARG
+#endif
+#if !defined(NV_DRM_ATOMIC_HELPER_SWAP_STATE_RETURN_INT)
+#define NV_DRM_ATOMIC_HELPER_SWAP_STATE_RETURN_INT
+#endif
+#endif
+
 #endif /* defined(__NVIDIA_DRM_CONFTEST_H__) */
--- a/conftest.sh
+++ b/conftest.sh
@@ -4747,6 +4747,56 @@
             compile_check_conftest "$CODE" "NV_DRM_APERTURE_REMOVE_CONFLICTING_FRAMEBUFFERS_HAS_NO_PRIMARY_ARG" "" "types"
         ;;
 
+        zone_device_page_init_has_pgmap_and_order_args)
+            #
+            # Determine if the zone_device_page_init() has two additional
+            # arguments
+            #
+            # This change was introduced by d245f9b4ab80
+            # ("mm/zone_device: support large zone device private folios")
+            #
+            # It was further amended in 9387a71ec62c
+            # (mm/zone_device: reinitialize large zone device private folios)
+            #
+            # both commits are in linux-next, expected in v6.19.
+            #
+            CODE="
+            #include <linux/memremap.h>
+
+            void init_page(void) {
+                struct page *page;
+                struct dev_pagemap *pgmap;
+
+                zone_device_page_init(page, pgmap, 0);
+            }"
+
+            compile_check_conftest "$CODE" "NV_ZONE_DEVICE_PAGE_INIT_HAS_PGMAP_AND_ORDER_ARGS" "" "types"
+        ;;
+
+        dev_pagemap_ops_has_folio_free)
+            #
+            # Determine if the zone device now uses a folio_free() as the callback
+            # function instead of page_free()
+            #
+            # This change was introduced by 3a5a06554566
+            # (mm/zone_device: rename page_free callback to folio_free)
+            #
+            # in linux-next, expected in v6.19.
+            #
+            CODE="
+            #include <linux/memremap.h>
+
+            void test_folio_free(struct folio *folio) {
+            }
+
+            void set_folio_free_ops(void) {
+                struct dev_pagemap_ops ops;
+                ops.folio_free = test_folio_free;
+            }"
+
+            compile_check_conftest "$CODE" "NV_PAGEMAP_OPS_HAS_FOLIO_FREE" "" "types"
+        ;;
+
         struct_page_has_zone_device_data)
             #
             # Determine if struct page has a 'zone_device_data' field.
"#;

const LLVM_DKMS_TOOLCHAIN: &str = r#"
if grep -q "CONFIG_CC_IS_CLANG=y" "/boot/config-$(uname -r)" 2>/dev/null; then
    echo "This kernel was built with LLVM/Clang: installing the toolchain DKMS needs..."
    apt install -y clang lld llvm
fi
"#;

pub(crate) fn nvidia_dkms_patch_commands() -> String {
    let mmap_patch = NV_MMAP_VMA_LOCK_PATCH.replace('\'', "'\\''");
    let string_h_patch = NV_STRING_H_PATCH.replace('\'', "'\\''");
    let dmem_patch = NV_DMEM_CGROUP_INIT_PATCH.replace('\'', "'\\''");
    let compat550_patch = NV_550_KERNEL72_COMPAT_PATCH.replace('\'', "'\\''");
    let compat590_patch = NV_590_KERNEL72_COMPAT_PATCH.replace('\'', "'\\''");

    format!(
        "for NVIDIA_SRC in /usr/src/nvidia-*/; do \
  for NV_SUB in '' 'kernel-open/'; do \
    NV_ROOT=\"${{NVIDIA_SRC}}${{NV_SUB}}\"; \
    NV_MMAP=\"${{NV_ROOT}}nvidia/nv-mmap.c\"; \
    if [ -f \"$NV_MMAP\" ]; then \
      if grep -q 'VM_REFCNT_EXCLUDE_READERS_FLAG' \"$NV_MMAP\" 2>/dev/null; then \
        echo \"NVIDIA VMA patch already applied in ${{NV_ROOT}} - skipping.\"; \
      elif ! grep -q 'VMA_LOCK_OFFSET' \"$NV_MMAP\" 2>/dev/null; then \
        echo \"${{NV_ROOT}} does not use VMA_LOCK_OFFSET - patch not needed.\"; \
      else \
        echo \"Applying NVIDIA VMA lock patch to ${{NV_ROOT}}...\"; \
        printf '%s' '{mmap_patch}' | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
        echo \"NVIDIA VMA patch applied.\" || \
        echo \"Warning: NVIDIA VMA patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
      fi; \
    fi; \
    NV_OSIF=\"${{NV_ROOT}}nvidia/os-interface.c\"; \
    if [ -f \"$NV_OSIF\" ]; then \
      if grep -q '#include <linux/string.h>' \"$NV_OSIF\" 2>/dev/null; then \
        echo \"NVIDIA string.h patch already applied in ${{NV_ROOT}} - skipping.\"; \
      else \
        echo \"Applying NVIDIA string.h patch to ${{NV_ROOT}}...\"; \
        printf '%s' '{string_h_patch}' | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
        echo \"NVIDIA string.h patch applied.\" || \
        echo \"Warning: NVIDIA string.h patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
      fi; \
      if grep -q 'NV_USE_DMEM_CGROUP_API' \"$NV_OSIF\" 2>/dev/null; then \
        if grep -q 'dmem_cgroup_register_region(&init' \"$NV_OSIF\" 2>/dev/null; then \
          echo \"NVIDIA dmem cgroup init patch already applied in ${{NV_ROOT}} - skipping.\"; \
        elif grep -q 'dmem_cgroup_register_region(size' \"$NV_OSIF\" 2>/dev/null; then \
          echo \"Applying NVIDIA dmem cgroup init patch to ${{NV_ROOT}}...\"; \
          printf '%s' '{dmem_patch}' | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
          echo \"NVIDIA dmem cgroup init patch applied.\" || \
          echo \"Warning: NVIDIA dmem cgroup init patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
        fi; \
      fi; \
    fi; \
    NV_LINUXH=\"${{NV_ROOT}}common/inc/nv-linux.h\"; \
    if [ -f \"$NV_LINUXH\" ] && [ -f \"$NV_OSIF\" ] && \
       grep -q 'NV_LINUX_OF_GPIO_H_PRESENT' \"$NV_LINUXH\" 2>/dev/null && \
       grep -q '^#include <linux/of_gpio.h>' \"$NV_LINUXH\" 2>/dev/null && \
       ! grep -q '__has_include(<linux/of_gpio.h>)' \"$NV_LINUXH\" 2>/dev/null && \
       grep -q 'strncpy(buf, current->comm, len - 1)' \"$NV_OSIF\" 2>/dev/null; then \
      NV_P550='{compat550_patch}'; \
      if printf '%s' \"$NV_P550\" | patch --fuzz=5 -p1 -d \"$NV_ROOT\" --dry-run > /dev/null 2>&1; then \
        echo \"Applying NVIDIA 550 kernel 7.2+ compatibility patch to ${{NV_ROOT}}...\"; \
        printf '%s' \"$NV_P550\" | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
        echo \"NVIDIA 550 compatibility patch applied.\" || \
        echo \"Warning: NVIDIA 550 compatibility patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
      else \
        echo \"Warning: NVIDIA 550 compatibility patch does not apply cleanly to ${{NV_ROOT}} - DKMS may fail.\"; \
      fi; \
    fi; \
    NV_UVMHMM=\"${{NV_ROOT}}nvidia-uvm/uvm_hmm.c\"; \
    if [ -f \"$NV_LINUXH\" ] && [ -f \"$NV_OSIF\" ] && [ -f \"$NV_UVMHMM\" ] && \
       ! grep -q 'NV_LINUX_OF_GPIO_H_PRESENT' \"$NV_LINUXH\" 2>/dev/null && \
       grep -q '^#include <linux/of_gpio.h>' \"$NV_LINUXH\" 2>/dev/null && \
       ! grep -q '__has_include(<linux/of_gpio.h>)' \"$NV_LINUXH\" 2>/dev/null && \
       grep -q 'strncpy(buf, current->comm, len - 1)' \"$NV_OSIF\" 2>/dev/null; then \
      NV_P590='{compat590_patch}'; \
      if printf '%s' \"$NV_P590\" | patch --fuzz=5 -p1 -d \"$NV_ROOT\" --dry-run > /dev/null 2>&1; then \
        echo \"Applying NVIDIA 590 kernel 7.2+ compatibility patch to ${{NV_ROOT}}...\"; \
        printf '%s' \"$NV_P590\" | patch --fuzz=5 -p1 -d \"$NV_ROOT\" && \
        echo \"NVIDIA 590 compatibility patch applied.\" || \
        echo \"Warning: NVIDIA 590 compatibility patch failed for ${{NV_ROOT}} - DKMS may fail.\"; \
      else \
        echo \"Warning: NVIDIA 590 compatibility patch does not apply cleanly to ${{NV_ROOT}} - DKMS may fail.\"; \
      fi; \
    fi; \
  done; \
done"
    )
}

// Installs an NVIDIA driver package straight from the Debian repositories —
// used by the 550 branch and (after the user enables Sid) the legacy
// 470/390/340 branches. Ported from drivers_tab.py's _on_nvidia_repo_clicked.
pub fn repo_script(package: &str) -> Result<String, String> {
    if !REPO_PACKAGES.contains(&package) {
        return Err(format!("Not an NVIDIA driver package of the Debian repositories: {package}"));
    }
    let patch = nvidia_dkms_patch_commands();
    let llvm = LLVM_DKMS_TOOLCHAIN;
    let script = format!(
        r#"set -e
echo "Installing NVIDIA driver from repository..."
echo "Removing NVIDIA DKMS modules from all kernels..."
for entry in $(dkms status | grep -i nvidia | awk -F'[:,]' '{{print $1"/"$2}}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
echo "Removing existing NVIDIA packages to prevent conflicts..."
apt purge -y 'nvidia-driver*' 'nvidia-kernel*' 'libnvidia*' 'nvidia-modprobe' 'nvidia-settings' 'nvidia-smi' 'nvidia-opencl*' 'nvidia-cuda*' 'cuda-drivers*' 'xserver-xorg-video-nvidia*' 2>/dev/null || true
apt update
apt install -y dkms build-essential linux-headers-$(uname -r)
{llvm}
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "Kernel build tree missing or broken, reinstalling headers..."
    apt install -y --reinstall linux-headers-$(uname -r) || true
fi
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "ERROR: no usable kernel build tree for $(uname -r)."
    echo "DKMS cannot build the NVIDIA module, so the driver is NOT installed."
    exit 1
fi
apt install -y {package} nvidia-smi nvidia-settings nvidia-modprobe libglu1-mesa || echo "First attempt failed (the DKMS build runs before the patch); patching the sources and retrying..."
echo "Applying NVIDIA DKMS compatibility patch for Soplos 7.x kernels..."
{patch}
dpkg --configure -a
apt install -y {package} nvidia-smi nvidia-settings nvidia-modprobe libglu1-mesa
echo "Rebuilding NVIDIA DKMS modules..."
dkms autoinstall || true
if ! dkms status | grep -qi "nvidia.*installed"; then
    echo "WARNING: the NVIDIA DKMS module did not build. Run 'dkms autoinstall' manually before rebooting."
fi
echo "Configuring GRUB with nvidia-drm.modeset=1..."
if ! grep -q "nvidia-drm.modeset=1" /etc/default/grub; then
    sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 nvidia-drm.modeset=1"/' /etc/default/grub
    update-grub
fi
if command -v dracut >/dev/null 2>&1; then
    mkdir -p /etc/dracut.conf.d
    echo 'omit_drivers+=" nouveau "' > /etc/dracut.conf.d/blacklist-nouveau.conf
    rm -f /etc/dracut.conf.d/nvidia.conf
    dracut --force
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
fi
echo "=== Installation completed ==="
echo "NVIDIA driver installed successfully. Restart the system to apply the changes."
"#
    );
    Ok(script)
}

// Installs an NVIDIA driver from the official NVIDIA CUDA repository — used
// by the 580/590/610 branches, each pinned to the Debian release NVIDIA
// actually publishes packages for. Ported from
// drivers_tab.py's _on_nvidia_cuda_repo_clicked.
pub fn cuda_script(version: &str) -> Result<String, String> {
    if !CUDA_BRANCHES.contains(&version) {
        return Err(format!("Not an NVIDIA driver branch of the CUDA repository: {version}"));
    }
    let (repo_setup, install_driver, repo_cleanup): (String, String, String) = match version {
        "580" => (
            r#"echo "deb [trusted=yes] https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/ /" \
    > /etc/apt/sources.list.d/cuda-debian12-x86_64.list
trap 'rm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list' EXIT"#
                .to_string(),
            "apt install -y nvidia-driver-pinning-580\napt install -y --allow-downgrades cuda-drivers-580".to_string(),
            "trap - EXIT\nrm -f /etc/apt/sources.list.d/cuda-debian12-x86_64.list\napt update -q".to_string(),
        ),
        "590" => (
            r#"TEMP_DIR=$(mktemp -d)
wget -q -O "$TEMP_DIR/cuda-keyring.deb" \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
if [ ! -s "$TEMP_DIR/cuda-keyring.deb" ]; then
    echo "ERROR: Failed to download cuda-keyring."
    rm -rf "$TEMP_DIR"
    exit 1
fi
dpkg -i "$TEMP_DIR/cuda-keyring.deb"
rm -rf "$TEMP_DIR""#
                .to_string(),
            "apt install -y nvidia-driver-pinning-590\napt install -y nvidia-open-590".to_string(),
            "apt purge -y cuda-keyring\nrm -f /etc/apt/sources.list.d/cuda-debian13-x86_64.list\napt update -q".to_string(),
        ),
        _ => (
            r#"TEMP_DIR=$(mktemp -d)
wget -q -O "$TEMP_DIR/cuda-keyring.deb" \
    https://developer.download.nvidia.com/compute/cuda/repos/debian13/x86_64/cuda-keyring_1.1-1_all.deb
if [ ! -s "$TEMP_DIR/cuda-keyring.deb" ]; then
    echo "ERROR: Failed to download cuda-keyring."
    rm -rf "$TEMP_DIR"
    exit 1
fi
dpkg -i "$TEMP_DIR/cuda-keyring.deb"
rm -rf "$TEMP_DIR""#
                .to_string(),
            "apt install -y nvidia-driver-pinning-610\napt install -y nvidia-open-610".to_string(),
            "apt purge -y cuda-keyring\nrm -f /etc/apt/sources.list.d/cuda-debian13-x86_64.list\napt update -q".to_string(),
        ),
    };
    let distro = if version == "580" { "debian12" } else { "debian13" };
    let patch = nvidia_dkms_patch_commands();
    let llvm = LLVM_DKMS_TOOLCHAIN;

    let script = format!(
        r#"set -e
echo "=== NVIDIA {version} Official CUDA Repository ({distro}) ==="
echo "[0/6] Removing NVIDIA DKMS modules from all kernels..."
for entry in $(dkms status | grep -i nvidia | awk -F'[:,]' '{{print $1"/"$2}}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
echo "[1/6] Removing existing NVIDIA/CUDA packages..."
rm -f /etc/dracut.conf.d/nvidia.conf
rm -f /etc/dracut.conf.d/blacklist-nouveau.conf
rm -f /etc/apt/apt.conf.d/99nvidia-sha1-exception
apt purge -y 'nvidia*' 'cuda*' 'libnvidia*' 2>/dev/null || true
apt -f install -y 2>/dev/null || true
echo "[2/6] Installing kernel headers and DKMS build dependencies..."
apt install -y dkms build-essential
apt install -y linux-headers-$(uname -r) || true
{llvm}
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "Kernel build tree missing or broken, reinstalling headers..."
    apt install -y --reinstall linux-headers-$(uname -r) || true
fi
if [ ! -e "/lib/modules/$(uname -r)/build/Makefile" ]; then
    echo "ERROR: no usable kernel build tree for $(uname -r)."
    echo "DKMS cannot build the NVIDIA module, so the driver is NOT installed."
    exit 1
fi
echo "[3/6] Setting up NVIDIA CUDA repository ({distro})..."
{repo_setup}
echo "[4/6] Installing NVIDIA Driver {version}..."
apt update
{install_driver} || echo "First attempt failed (the DKMS build runs before the patch); patching the sources and retrying..."
echo "[+] Applying NVIDIA DKMS compatibility patch for Soplos 7.x kernels..."
{patch}
dpkg --configure -a
{install_driver}
{repo_cleanup}
echo "[+] Rebuilding NVIDIA DKMS modules..."
dkms autoinstall || true
if ! dkms status | grep -qi "nvidia.*installed"; then
    echo "WARNING: the NVIDIA DKMS module did not build. Run 'dkms autoinstall' manually before rebooting."
fi
echo "[5/6] Configuring dracut and regenerating initramfs..."
if command -v dracut >/dev/null 2>&1; then
    mkdir -p /etc/dracut.conf.d
    echo 'omit_drivers+=" nouveau "' > /etc/dracut.conf.d/blacklist-nouveau.conf
    rm -f /etc/dracut.conf.d/nvidia.conf
    dracut --force
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
fi
echo "[+] Configuring GRUB with nvidia-drm.modeset=1..."
if ! grep -q "nvidia-drm.modeset=1" /etc/default/grub; then
    sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 nvidia-drm.modeset=1"/' /etc/default/grub
fi
update-grub
echo "[6/6] Done."
echo "NVIDIA driver {version} installed. Restart to apply changes."
"#
    );
    Ok(script)
}

// Fully removes NVIDIA/CUDA — shared by every "uninstall" driver button and
// by switching to nouveau. Ported from drivers_tab.py's
// _build_nvidia_uninstall_script.
pub fn uninstall_script() -> String {
    let script = r#"echo "=== Removing all NVIDIA drivers ==="
echo "[0/7] Removing NVIDIA DKMS modules from all kernels..."
for entry in $(dkms status | grep -i nvidia | awk -F'[:,]' '{print $1"/"$2}' | tr -d ' '); do
    dkms remove --force "$entry" --all 2>/dev/null || true
done
for kver in $(ls /lib/modules/); do
    rm -f /lib/modules/$kver/updates/dkms/nvidia*.ko*
    rm -f /lib/modules/$kver/updates/dkms/nvidia-*.ko*
    depmod -a "$kver" 2>/dev/null || true
done
rm -rf /var/lib/dkms/nvidia*
echo "[1/7] Fixing any interrupted dpkg state..."
dpkg --configure -a 2>/dev/null || true
echo "[2/7] Removing NVIDIA and CUDA packages..."
NVIDIA_PKGS=$(dpkg -l | grep -iE '^[a-z]+[[:space:]]+(nvidia|libnvidia|cuda)' | awk '{print $2}' | tr '\n' ' ')
if [ -n "$NVIDIA_PKGS" ]; then
    apt purge -y $NVIDIA_PKGS 2>/dev/null || dpkg --purge --force-depends $NVIDIA_PKGS
else
    echo "No NVIDIA/CUDA packages found."
fi
echo "[3/7] Removing NVIDIA CUDA repository sources..."
rm -f /etc/apt/sources.list.d/cuda-*.list
rm -f /etc/apt/sources.list.d/nvidia*.list
rm -f /usr/share/keyrings/cuda-*.gpg
rm -f /usr/share/keyrings/nvidia*.gpg
apt update -q
echo "[4/7] Removing NVIDIA dracut configuration..."
rm -f /etc/dracut.conf.d/nvidia.conf
rm -f /etc/dracut.conf.d/blacklist-nouveau.conf
rm -f /etc/apt/apt.conf.d/99nvidia-sha1-exception
echo "[5/7] Restoring GRUB defaults..."
sed -i 's/ nvidia-drm.modeset=1//' /etc/default/grub 2>/dev/null || true
update-grub 2>/dev/null || true
echo "[6/7] Regenerating initramfs..."
if command -v dracut >/dev/null 2>&1; then
    dracut --force
elif command -v update-initramfs >/dev/null 2>&1; then
    update-initramfs -u
fi
echo "[7/7] Done. Restart the system before installing a new driver version."
"#
    .to_string();
    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn is_valid_shell(script: &str, name: &str) {
        let file = std::env::temp_dir().join(format!("soplos-store-test-{}-{name}.sh", std::process::id()));
        std::fs::write(&file, script).unwrap();
        let out = Command::new("bash").arg("-n").arg(&file).output().unwrap();
        let _ = std::fs::remove_file(&file);
        assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
    }

    #[test]
    fn every_script_the_helper_can_run_is_valid_shell() {
        for package in REPO_PACKAGES {
            is_valid_shell(&repo_script(package).unwrap(), package);
        }
        for branch in CUDA_BRANCHES {
            is_valid_shell(&cuda_script(branch).unwrap(), branch);
        }
        is_valid_shell(&uninstall_script(), "remove");
    }

    #[test]
    fn only_the_listed_branches_are_accepted() {
        assert!(cuda_script("999").is_err());
        assert!(cuda_script("590; rm -rf /").is_err());
        assert!(cuda_script("").is_err());
        assert!(repo_script("nvidia-driver; reboot").is_err());
        assert!(repo_script("bash").is_err());
        assert!(repo_script("nvidia-driver").is_ok());
    }

    #[test]
    fn the_scripts_carry_the_kernel_patches() {
        let script = cuda_script("590").unwrap();
        assert!(script.contains("NV_P590"), "Fix 7 for the 590 open modules");
        assert!(script.contains("NV_P550"), "Fix 6 for the 550");
        assert!(script.contains("dkms autoinstall"));
    }
}
