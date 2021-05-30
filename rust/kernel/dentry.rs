use alloc::boxed::Box;
use core::{
    mem,
    ops::{Deref, DerefMut},
};

use crate::bindings;
pub type Inode = bindings::inode;

#[repr(transparent)]
pub struct Dentry(bindings::dentry);

impl Deref for Dentry {
    type Target = bindings::dentry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Dentry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Dentry {
    pub fn as_raw_inner(self: Box<Self>) -> *mut bindings::dentry {
        Box::into_raw(self) as *mut _
    }

    pub fn make_root(inode: &mut Inode) -> Option<&mut Self> {
        unsafe {
            Some(bindings::d_make_root(inode as *mut _))
                .filter(|p| !p.is_null())
                .map(|p| mem::transmute(&mut *p))
        }
    }

    pub fn lookup(&mut self, query: *const bindings::qstr) -> Option<&mut Self> {
        unsafe {
            Some(bindings::d_lookup(&mut self.0 as *mut _, query))
                .filter(|p| !p.is_null())
                .map(|p| mem::transmute(&mut *p))
        }
    }

    pub fn get(&mut self) {
        // couldn't find in bindings, lol
        unimplemented!()
    }

    pub fn put(&mut self) {
        unsafe {
            bindings::dput(&mut self.0 as *mut _);
        }
    }

    pub fn drop_dentry(&mut self) {
        unsafe {
            bindings::d_drop(&mut self.0 as *mut _);
        }
    }

    pub fn delete_dentry(&mut self) {
        unsafe {
            bindings::d_delete(&mut self.0 as *mut _);
        }
    }

    pub fn add(&mut self, inode: &mut Inode) {
        unsafe {
            bindings::d_add(&mut self.0 as *mut _, inode as *mut _);
        }
    }

    pub fn instantiate(&mut self, inode: &mut Inode) {
        unsafe {
            bindings::d_instantiate(&mut self.0 as *mut _, inode as *mut _);
        }
    }
}
