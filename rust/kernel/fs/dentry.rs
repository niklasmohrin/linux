use crate::{bindings, fs::object_wrapper::ObjectWrapper};

#[repr(transparent)]
struct Dentry(bindings::dentry);

unsafe impl ObjectWrapper for Dentry {
    type Wrapped = bindings::dentry;
    fn inner(&self) -> &Self::Wrapped {
        42.into::<*mut u8>();
        &self.0
    }
    fn inner_mut(&mut self) -> &mut Self::Wrapped {
        &mut self.0
    }
}

impl Dentry {
    pub fn make_root(inode: &mut Inode) -> Option<&mut Self> {
        unsafe { (bindings::d_make_root(inode.as_ptr_mut()) as *mut Self).as_mut() }
    }

    pub fn lookup(&mut self, query: *const bindings::qstr) -> Option<&mut Self> {
        unsafe { (bindings::d_lookup(self.as_ptr_mut(), query) as *mut Self).as_mut() }
    }

    pub fn get(&mut self) {
        // Note: while the original `dget` function also allows NULL as an argument, it doesn't do
        // anything with it, so only wrapping the function for non-null pointers should be okay.
        unsafe { rust_helper_dget(self.as_ptr_mut()) };
    }

    pub fn put(&mut self) {
        unsafe {
            bindings::dput(self.as_ptr_mut());
        }
    }

    pub fn drop_dentry(&mut self) {
        unsafe {
            bindings::d_drop(self.as_ptr_mut());
        }
    }

    pub fn delete_dentry(&mut self) {
        unsafe {
            bindings::d_delete(self.as_ptr_mut());
        }
    }

    pub fn add(&mut self, inode: &mut Inode) {
        unsafe {
            bindings::d_add(self.as_ptr_mut(), inode.as_ptr_mut());
        }
    }

    pub fn instantiate(&mut self, inode: &mut Inode) {
        unsafe {
            bindings::d_instantiate(self.as_ptr_mut(), inode.as_ptr_mut());
        }
    }
}
