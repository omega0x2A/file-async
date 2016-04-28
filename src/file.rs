use std::path::Path;

use async_data::AsyncData;

use win_api_helper::create_file_async;
use win_api_helper::create_io_completion_port;

use tools::write_file_async_data;
use tools::read_file_async_data;

use io_worker::init_static_completion_port_once;

use winapi::HANDLE;
use winapi::GENERIC_WRITE;
use winapi::GENERIC_READ;
use winapi::OPEN_EXISTING;
use winapi::CREATE_NEW;
use winapi::DWORD;
use kernel32::CloseHandle;

// -----------------------------------------------------------------------------
pub struct File {
    file: HANDLE,
    cluster_size: usize,
}

// -----------------------------------------------------------------------------
impl File {
    // -------------------------------------------------------------------------
    pub fn create<P: AsRef<Path>>(path: P) -> Result<File, String> {
        File::generic_create(path, CREATE_NEW)
    }

    // -------------------------------------------------------------------------
    pub fn open<P: AsRef<Path>>(path: P) -> Result<File, String> {
        File::generic_create(path, OPEN_EXISTING)
    }

    // -------------------------------------------------------------------------
    fn generic_create<P: AsRef<Path>>(path: P, flags: DWORD) -> Result<File, String> {
        let file = try!(create_file_async(path, GENERIC_WRITE | GENERIC_READ, flags));
        let io_completion_port = try!(init_static_completion_port_once());
		
        try!(create_io_completion_port(file, io_completion_port, 0, 0));
        Ok(File {
            file: file,
            cluster_size: File::get_cluster_size()})
    }
    
    // -----------------------------------------------------------------------------
    pub fn get_cluster_size() -> usize {
    	1024	
    }
    
    // -----------------------------------------------------------------------------
    pub fn write_all(&mut self,
                     mut buff: Vec<u8>, 
                     callback: Box<Fn(Result<(), String>)>) {
        let byte_to_write = buff.len();
        self.adjust_write_buffer(&mut buff);
        let async_data = Box::new(AsyncData::new_write_data(self.file,
                                                            buff,
                                                            byte_to_write,
                                                            callback));

        write_file_async_data(self.file, async_data);
    }

    // -----------------------------------------------------------------------------
    pub fn read_all_with_buffer_size(&mut self,
                                     approximate_read_size: usize,
                                     callback: Box<Fn(Result<&[u8], String>)>) {
        let read_size = self.compute_buffer_size(approximate_read_size);
        let async_data = Box::new(AsyncData::new_read_data(self.file, read_size, callback));

        read_file_async_data(self.file, async_data);
    }

    // -----------------------------------------------------------------------------
    pub fn read_all(&mut self, callback: Box<Fn(Result<&[u8], String>)>) {
        self.read_all_with_buffer_size(1024, callback)
    }

    // -----------------------------------------------------------------------------
    fn compute_buffer_size(&self, approximate_buffer_size: usize) -> usize {
        let buffer_size = (approximate_buffer_size / self.cluster_size) * self.cluster_size;

        if buffer_size < approximate_buffer_size {
            buffer_size + self.cluster_size
        } else {
            buffer_size
        }
    }

    // -----------------------------------------------------------------------------
    fn adjust_write_buffer(&self, buff: &mut Vec<u8>) {
        let new_size = ((buff.len() + self.cluster_size - 1) / self.cluster_size) *
                       self.cluster_size;
        buff.reserve(new_size);

        unsafe {
            buff.set_len(new_size);
        }
    }
}

// -----------------------------------------------------------------------------
impl Drop for File {
    
    // -------------------------------------------------------------------------
    fn drop(&mut self) {
        unsafe {
            println!("Drop file");
            if CloseHandle(self.file) == 0 {
                panic!("Cannot close file");
            }
        }
    }
}
