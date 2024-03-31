#![allow(dead_code)]

const PAGE_SIZE: usize = 4096;

#[cfg(unix)]
pub fn write_function(data: Vec<u8>) -> extern "C" fn(*mut u8) -> i32 {
    let size = data.len();
    let page_count = size.div_ceil(PAGE_SIZE);
    let mem_size = page_count * PAGE_SIZE;
    unsafe {
        let mut raw_ptr: *mut libc::c_void = std::mem::zeroed();
        libc::posix_memalign(&mut raw_ptr, PAGE_SIZE, mem_size);
        libc::mprotect(raw_ptr, mem_size, libc::PROT_READ | libc::PROT_WRITE);
        libc::memcpy(raw_ptr, std::mem::transmute(data.as_ptr()), mem_size);
        libc::mprotect(raw_ptr, mem_size, libc::PROT_EXEC | libc::PROT_READ);
        std::mem::transmute(raw_ptr)
    }
}

#[cfg(windows)]
pub fn write_function(data: Vec<u8>) -> extern "C" fn(*mut u8) -> i32 {
    use windows_sys::Win32::System::Memory::*;

    let size = data.len();
    let page_count = size.div_ceil(PAGE_SIZE);
    let mem_size = page_count * PAGE_SIZE;
    unsafe {
        let raw_ptr: *mut core::ffi::c_void;
        let mut out: u32 = 0;
        raw_ptr = VirtualAlloc(std::ptr::null_mut(), mem_size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);

        if raw_ptr == 0 as *mut core::ffi::c_void {
            panic!("Could not allocate memory");
        }

        libc::memcpy(raw_ptr, std::mem::transmute(data.as_ptr()), mem_size);
        VirtualProtect(raw_ptr, mem_size, PAGE_EXECUTE_READ, std::ptr::addr_of_mut!(out));
        std::mem::transmute(raw_ptr)
    }
}
