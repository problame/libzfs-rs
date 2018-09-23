extern crate libzfs_bindings;

use libzfs_bindings::*;

pub struct LibZFSHandle {
    h: *mut libzfs_handle_t
}

impl LibZFSHandle {
    pub fn init() -> Result<LibZFSHandle,()> {
        unsafe {
            let h = libzfs_init();
            if h == std::ptr::null_mut() {
                return Err(());
            }
            Ok(LibZFSHandle{h})
        }
    }

    pub fn roots(&self) -> Result<Vec<ZFSHandle>,()> {
        unsafe extern "C" fn iter_cb(zh: *mut zfs_handle_t, v: *mut std::os::raw::c_void) -> i32 {
                let v = (v as *mut Vec<ZFSHandle>).as_mut().unwrap();
                v.push(ZFSHandle::from_ptr(zh));
                return 0;
        }
        unsafe {
            let mut v = Vec::new();
            let ret = zfs_iter_root(self.h, Some(iter_cb), &mut v as *mut Vec<ZFSHandle> as *mut c_void);
            if ret != 0 {
                return Err(());
            }
            return Ok(v);
        }
    }

}

impl Drop for LibZFSHandle {
    fn drop(&mut self) {
        println!("dropping handle");
        unsafe {
            libzfs_fini(self.h);
            self.h = std::ptr::null_mut();
        }
    }
}

pub struct ZFSHandle {
    h: *mut zfs_handle_t
}

impl ZFSHandle {
    fn from_ptr(h: *mut zfs_handle_t) -> ZFSHandle {
        ZFSHandle{h}
    }

    pub fn get_name(&self) -> Result<&std::ffi::CStr,()>  {
        unsafe {
            match  zfs_get_name(self.h).as_ref() {
                Some(cptr) => Ok(std::ffi::CStr::from_ptr(cptr)),
                None => Err(())
            }
        }
    }

    pub fn get_type(&self) -> Result<zfs_type_t::Type,()> {
        unsafe {
            match zfs_get_type(self.h) {
                0 => Err(()),
                t => Ok(t),
            }
        }
    }

    pub fn children(&self) -> impl Iterator<Item=ZFSHandle> {
        Children::init(self.h)
    }

}

impl Drop for ZFSHandle {
    fn drop(&mut self) {
        unsafe {
            zfs_close(self.h);
        }
    }
}

#[derive(Copy,Clone)]
struct Send_zfs_handle_t(*mut zfs_handle_t);

unsafe impl std::marker::Send for Send_zfs_handle_t {}

pub struct Children {
    receiver: std::sync::mpsc::Receiver<Send_zfs_handle_t>,
    h: Send_zfs_handle_t,
}

use std::os::raw::c_void;

impl Children {
    fn init(h: *mut zfs_handle_t) -> Box<Children> {

        let h = Send_zfs_handle_t(h);

        let (mut sender, mut receiver) = std::sync::mpsc::sync_channel(0);

        let iterator_thread = std::thread::spawn(move || {
            unsafe {
                 unsafe extern "C" fn iter_cb(zh: *mut zfs_handle_t, sender_ptr: *mut std::os::raw::c_void) -> i32 {
                    unsafe {
                        let sender = (sender_ptr as *mut std::sync::mpsc::SyncSender<Send_zfs_handle_t>).as_ref().unwrap();
                        sender.send(Send_zfs_handle_t(zh));
                    }
                    return 0;
                }
                zfs_iter_children(h.0, Some(iter_cb), &mut sender as *mut _ as *mut c_void);
            }       
        });

        Box::new(Children{h, receiver})
    }
}

impl Iterator for Children {
    type Item = ZFSHandle;
    fn next(&mut self) -> std::option::Option<ZFSHandle> {
        match self.receiver.recv() {
            Ok(h) => Some(ZFSHandle::from_ptr(h.0)),
            Err(e) => None,
        }
    }
}

struct RecursiveChildren {
    stack: Vec<Box<Iterator<Item=ZFSHandle>>>,
}

impl RecursiveChildren {
    fn init(roots: Box<Iterator<Item=ZFSHandle>>) -> RecursiveChildren {
        RecursiveChildren{stack: vec![roots]}
    }
}

impl Iterator for RecursiveChildren {
    type Item = ZFSHandle;
    fn next(&mut self) -> Option<ZFSHandle> {
        while !self.stack.is_empty() {
            let next = {
                let cur = match self.stack.last_mut() {
                    Some(it) => it,
                    None => return None, // iterator empty 
                };
                // use cur
                cur.next()
            };
            if let Some(zh) = next {
                // println!("pushing stack");
                self.stack.push(Box::new(zh.children()));
                return Some(zh);
            }
            // println!("popping stack");
            self.stack.pop();
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_dropping() {
        let h = LibZFSHandle::init();
        assert!(false);
    }

    #[test]
    fn test_recursive_children() {
        let h = LibZFSHandle::init().unwrap();
        let roots = h.roots().unwrap();
        let roots = roots.into_iter();
        let rec = RecursiveChildren::init(Box::new(roots));
        for fs in rec {
            if fs.get_type().unwrap() == libzfs_bindings::zfs_type_t::ZFS_TYPE_FILESYSTEM {
                println!("fs={:?}", fs.get_name());
            }
        }
        assert!(false);
    }

}