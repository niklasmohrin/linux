// SPDX-License-Identifier: GPL-2.0

#include <asm/unaligned.h>
#include <linux/backing-dev.h>
#include <linux/buffer_head.h>
#include <linux/bug.h>
#include <linux/build_bug.h>
#include <linux/errname.h>
#include <linux/gfp.h>
#include <linux/highmem.h>
#include <linux/mutex.h>
#include <linux/pagemap.h>
#include <linux/sched/signal.h>
#include <linux/uaccess.h>
#include <linux/uio.h>

void rust_helper_BUG(void)
{
	BUG();
}

unsigned long rust_helper_copy_from_user(void *to, const void __user *from,
					 unsigned long n)
{
	return copy_from_user(to, from, n);
}

unsigned long rust_helper_copy_to_user(void __user *to, const void *from,
				       unsigned long n)
{
	return copy_to_user(to, from, n);
}

unsigned long rust_helper_clear_user(void __user *to, unsigned long n)
{
	return clear_user(to, n);
}

void rust_helper_spin_lock_init(spinlock_t *lock, const char *name,
				struct lock_class_key *key)
{
#ifdef CONFIG_DEBUG_SPINLOCK
	__spin_lock_init(lock, name, key);
#else
	spin_lock_init(lock);
#endif
}
EXPORT_SYMBOL_GPL(rust_helper_spin_lock_init);

void rust_helper_spin_lock(spinlock_t *lock)
{
	spin_lock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_spin_lock);

void rust_helper_spin_unlock(spinlock_t *lock)
{
	spin_unlock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_spin_unlock);

void rust_helper_init_wait(struct wait_queue_entry *wq_entry)
{
	init_wait(wq_entry);
}
EXPORT_SYMBOL_GPL(rust_helper_init_wait);

int rust_helper_current_pid(void)
{
	return current->pid;
}
EXPORT_SYMBOL_GPL(rust_helper_current_pid);

int rust_helper_signal_pending(void)
{
	return signal_pending(current);
}
EXPORT_SYMBOL_GPL(rust_helper_signal_pending);

struct page *rust_helper_alloc_pages(gfp_t gfp_mask, unsigned int order)
{
	return alloc_pages(gfp_mask, order);
}
EXPORT_SYMBOL_GPL(rust_helper_alloc_pages);

void *rust_helper_kmap(struct page *page)
{
	return kmap(page);
}
EXPORT_SYMBOL_GPL(rust_helper_kmap);

void rust_helper_kunmap(struct page *page)
{
	return kunmap(page);
}
EXPORT_SYMBOL_GPL(rust_helper_kunmap);

int rust_helper_cond_resched(void)
{
	return cond_resched();
}
EXPORT_SYMBOL_GPL(rust_helper_cond_resched);

size_t rust_helper_copy_from_iter(void *addr, size_t bytes, struct iov_iter *i)
{
	return copy_from_iter(addr, bytes, i);
}
EXPORT_SYMBOL_GPL(rust_helper_copy_from_iter);

size_t rust_helper_copy_to_iter(const void *addr, size_t bytes,
				struct iov_iter *i)
{
	return copy_to_iter(addr, bytes, i);
}
EXPORT_SYMBOL_GPL(rust_helper_copy_to_iter);

bool rust_helper_is_err(__force const void *ptr)
{
	return IS_ERR(ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_is_err);

long rust_helper_ptr_err(__force const void *ptr)
{
	return PTR_ERR(ptr);
}
EXPORT_SYMBOL_GPL(rust_helper_ptr_err);

const char *rust_helper_errname(int err)
{
	return errname(err);
}

void rust_helper_mutex_lock(struct mutex *lock)
{
	mutex_lock(lock);
}
EXPORT_SYMBOL_GPL(rust_helper_mutex_lock);

/* We use bindgen's --size_t-is-usize option to bind the C size_t type
 * as the Rust usize type, so we can use it in contexts where Rust
 * expects a usize like slice (array) indices. usize is defined to be
 * the same as C's uintptr_t type (can hold any pointer) but not
 * necessarily the same as size_t (can hold the size of any single
 * object). Most modern platforms use the same concrete integer type for
 * both of them, but in case we find ourselves on a platform where
 * that's not true, fail early instead of risking ABI or
 * integer-overflow issues.
 *
 * If your platform fails this assertion, it means that you are in
 * danger of integer-overflow bugs (even if you attempt to remove
 * --size_t-is-usize). It may be easiest to change the kernel ABI on
 * your platform such that size_t matches uintptr_t (i.e., to increase
 * size_t, because uintptr_t has to be at least as big as size_t).
*/
static_assert(sizeof(size_t) == sizeof(uintptr_t) &&
		      __alignof__(size_t) == __alignof__(uintptr_t),
	      "Rust code expects C size_t to match Rust usize");

void rust_helper_dget(struct dentry *dentry)
{
	dget(dentry);
}
EXPORT_SYMBOL_GPL(rust_helper_dget);

void rust_helper_mapping_set_unevictable(struct address_space *mapping)
{
	mapping_set_unevictable(mapping);
}
EXPORT_SYMBOL_GPL(rust_helper_mapping_set_unevictable);

void rust_helper_mapping_set_gfp_mask(struct address_space *mapping, gfp_t mask)
{
	mapping_set_gfp_mask(mapping, mask);
}
EXPORT_SYMBOL_GPL(rust_helper_mapping_set_gfp_mask);

const gfp_t RUST_HELPER_GFP_HIGHUSER = GFP_HIGHUSER;
EXPORT_SYMBOL_GPL(RUST_HELPER_GFP_HIGHUSER);

int rust_helper_generic_cont_expand_simple(struct inode *inode, loff_t size)
{
	return generic_cont_expand_simple(inode, size);
}
EXPORT_SYMBOL_GPL(rust_helper_generic_cont_expand_simple);

int rust_helper_sync_mapping_buffers(struct address_space *mapping)
{
	return sync_mapping_buffers(mapping);
}
EXPORT_SYMBOL_GPL(rust_helper_sync_mapping_buffers);

void rust_helper_inode_lock(struct inode *inode)
{
	inode_lock(inode);
}
EXPORT_SYMBOL_GPL(rust_helper_inode_lock);

void rust_helper_inode_unlock(struct inode *inode)
{
	inode_unlock(inode);
}
EXPORT_SYMBOL_GPL(rust_helper_inode_unlock);

void rust_helper_mark_inode_dirty(struct inode *inode)
{
	mark_inode_dirty(inode);
}
EXPORT_SYMBOL_GPL(rust_helper_mark_inode_dirty);

loff_t rust_helper_i_size_read(const struct inode *inode)
{
	return i_size_read(inode);
}
EXPORT_SYMBOL_GPL(rust_helper_i_size_read);

struct buffer_head *rust_helper_sb_bread(struct super_block *sb, sector_t block)
{
	return sb_bread(sb, block);
}
EXPORT_SYMBOL_GPL(rust_helper_sb_bread);

void rust_helper_brelse(struct buffer_head *bh)
{
	brelse(bh);
}
EXPORT_SYMBOL_GPL(rust_helper_brelse);

u16 rust_helper_get_unaligned_le16(const void *p)
{
	return get_unaligned_le16(p);
}
EXPORT_SYMBOL_GPL(rust_helper_get_unaligned_le16);

u32 rust_helper_get_unaligned_le32(const void *p)
{
	return get_unaligned_le32(p);
}
EXPORT_SYMBOL_GPL(rust_helper_get_unaligned_le32);

u16 rust_helper_le16_to_cpu(const u16 x)
{
	return le16_to_cpu(x);
}
EXPORT_SYMBOL_GPL(rust_helper_le16_to_cpu);

u16 rust_helper_cpu_to_le16(const u16 x)
{
	return cpu_to_le16(x);
}
EXPORT_SYMBOL_GPL(rust_helper_cpu_to_le16);

u32 rust_helper_le32_to_cpu(const u32 x)
{
	return le32_to_cpu(x);
}
EXPORT_SYMBOL_GPL(rust_helper_le32_to_cpu);

#if !defined(CONFIG_ARM)
// See https://github.com/rust-lang/rust-bindgen/issues/1671
static_assert(__builtin_types_compatible_p(size_t, uintptr_t),
	      "size_t must match uintptr_t, what architecture is this??");
#endif

long rust_helper_congestion_wait(int sync, long timeout)
{
	return congestion_wait(sync, timeout);
}
EXPORT_SYMBOL_GPL(rust_helper_congestion_wait);

const long RUST_HELPER_HZ = HZ;
EXPORT_SYMBOL_GPL(RUST_HELPER_HZ);
