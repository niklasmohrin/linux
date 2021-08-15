use crate::{bindings, fs::BuildVtable, Result};
use alloc::boxed::Box;
use core::{
    mem,
    ops::{Deref, DerefMut},
};

#[repr(transparent)]
pub struct AddressSpace(bindings::address_space);

impl Deref for AddressSpace {
    type Target = bindings::address_space;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for AddressSpace {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl AsRef<AddressSpace> for bindings::address_space {
    fn as_ref(&self) -> &AddressSpace {
        unsafe { mem::transmute(self) }
    }
}
impl AsMut<AddressSpace> for bindings::address_space {
    fn as_mut(&mut self) -> &mut AddressSpace {
        unsafe { mem::transmute(self) }
    }
}

impl AddressSpace {
    pub fn as_ptr_mut(&mut self) -> *mut bindings::address_space {
        self.deref_mut() as *mut _
    }

    pub fn set_address_space_operations<Ops: BuildVtable<bindings::address_space_operations>>(
        &mut self,
        ops: Ops,
    ) -> Result {
        self.a_ops = Ops::build_vtable();
        self.private_data = Box::into_raw(Box::try_new(ops)?).cast();
        Ok(())
    }
}
