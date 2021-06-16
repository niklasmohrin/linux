module! {
    type: BS2FAT,
    name: b"bs2fat",
    author: b"Rust for Linux Contributors",
    description: b"MS-DOS filesystem support",
    license: b"GPL v2",
}

/* Characters that are undesirable in an MS-DOS file name */
const BAD_CHARS: [i8] = "*?<>|\"";
const BAD_IF_STRICT: [i8] = "+=,; ";

struct BS2Fat;

impl FileSystemBase for BS2Fat {
    const NAME: &'static CStr = kernel::cstr!("bs2fat");
    const FS_FLAGS: c_int = bindings::FS_USERNS_MOUNT as _;
    const OWNER: *mut bindings::module = ptr::null_mut();

    fn mount(
        _fs_type: &'_ mut FileSystemType,
        flags: c_int,
        _device_name: &CStr,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry> {
        libfs_functions::mount_bdev::<Self>(flags, data)
    }

    fn kill_super(sb: &mut SuperBlock) {
        
    }

    fn fill_super(
        sb: &mut SuperBlock,
        _data: Option<&mut Self::MountOptions>,
        _silent: c_int,
    ) -> Result {

        sb.s_magic = BS2FAT_MAGIC;
        
    }
}

kernel::declare_fs_type!(BS2Ramfs, BS2RAMFS_FS_TYPE);

impl KernelModule for BS2Fat {
    fn init() -> Result<Self> {
        pr_emerg!("BSFat in action");
        Self::register().map(move |_| Self)

        // Irgendwas mit caching...
    }
}

impl Drop for BS2Fat {
    fn drop(&mut self) {
        let _ = Self::unregister();
        pr_info!("BSFat out of action");
    }
}

    fn init() -> Result<Self> {
        pr_emerg!("BSFat in action");
        Self::register().map(move |_| Self)

        // Irgendwas mit caching...
    }
}

impl Drop for BS2Fat {
    fn drop(&mut self) {
        let _ = Self::unregister();
        pr_info!("BSFat out of action");
    }
}

