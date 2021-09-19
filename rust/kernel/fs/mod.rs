// FIXME
#![allow(missing_docs)]

pub mod address_space;
pub mod address_space_operations;
pub mod dentry;
pub mod file_system;
pub mod inode;
pub mod inode_operations;
pub mod kiocb;
pub mod libfs_functions;
pub mod super_block;
pub mod super_operations;

pub trait BuildVtable<T> {
    fn build_vtable() -> &'static T;
}

#[macro_export]
macro_rules! declare_c_vtable {
    ($O:ident, $T:ty, $val:expr $(,)?) => {
        pub struct $O;
        impl $crate::fs::BuildVtable<$T> for $O {
            fn build_vtable() -> &'static $T {
                unsafe { &($val) }
            }
        }
    };
}
