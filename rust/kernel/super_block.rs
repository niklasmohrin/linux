use crate::bindings;

pub trait SuperBlock {
    type Inode;

    const MAGIC: u64;
    const BLOCKSIZE: u64;
    const BLOCKSIZE_BITS: u8;

    pub fn as_inner(&mut self) -> &mut bindings::super_block;
    pub fn from_inner(sb: &mut bindings::super_block) -> &mut Self;

    const statfs_implemented: bool = false; // TODO add a derive macro to auto-generate these
    pub fn statfs(&mut self, dentry: Box<Dentry>, buf: Box<KStatfs>) -> KernelResult<()> {
        unreachable!()
    }
    
    unsafe extern "C" fn statfs_raw(dentry: *mut bindings::dentry, buf: *mut bindings::kstatfs) -> i32 {
        let dentry = Dentry::from_raw(*dentry);
        let buf = KStatsfs::from_raw(*buf);
        let res = Self::statsfs(dentry, buf);
        let _ = Dentry::into_raw(dentry);
        let _ = KStatsfs::into_raw(buf);
        res
    }

    const drop_inode_implemented: bool = false;
    pub fn drop_inode(inode: Box<Inode>) -> KernelResult {
        unreachable!()
    }

    unsafe extern "C" fn drop_inode_raw(inode: *mut bindings::inode) -> i32 {
        let inode = Inode::from_raw(*inode);
        let res = Self::drop_inode(inode);
        let _ = Inode::into_raw(inode);
        res
    }

    const alloc_inode_implemented: bool = false;
    pub fn alloc_inode(&mut self) -> Box<Inode> {
        unreachable!()
    }

    unsafe extern "C" fn alloc_inode(sb: *mut bindings::super_block) -> *mut bindings::inode {
        let inode = Inode::from_raw(*inode);
        let res = Self::from_inner(sb).alloc_inode(inode);
        let _ = Inode::into_raw(inode);
        res
    }

    // TODO add all the lots of other methods
    
}
