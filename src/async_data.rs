use winapi::OVERLAPPED;
use winapi::HANDLE;
use std::ptr::null_mut;

// -----------------------------------------------------------------------------
pub struct WriteData {
    pub bytes_to_write: usize,
    pub callback: Box<Fn(Result<(), String>)>,
}

// -----------------------------------------------------------------------------
pub struct ReadData {
    pub read_size: usize,
    pub callback: Box<Fn(Result<&[u8], String>)>,
}

// -----------------------------------------------------------------------------
pub enum DataType {
    Write(WriteData),
    Read(ReadData),
}

// -----------------------------------------------------------------------------
#[repr(C)]
pub struct AsyncData {
    pub overlapped: OVERLAPPED,
    pub file_handle: HANDLE,
    pub buffer: Vec<u8>,
    pub data_type: DataType,
}

// -----------------------------------------------------------------------------
impl AsyncData {
    // -------------------------------------------------------------------------
    pub fn new_write_data(file_handle: HANDLE,
                          buffer: Vec<u8>,
                          bytes_to_write: usize,
                          callback: Box<Fn(Result<(), String>)>)
                          -> AsyncData {
                          	println!("Create async data w");
        AsyncData {
            overlapped: AsyncData::create_overlapped(),
            file_handle: file_handle,
            buffer: buffer,
            data_type: DataType::Write(WriteData {
                bytes_to_write: bytes_to_write,
                callback: callback,
            }),
        }
    }

    // -------------------------------------------------------------------------
    pub fn new_read_data(file_handle: HANDLE,
                         read_size: usize,
                         callback: Box<Fn(Result<&[u8], String>)>)
                         -> AsyncData {
                         	println!("Create async data r");
        let mut buffer = Vec::<u8>::with_capacity(read_size);
        unsafe {
            buffer.set_len(read_size);
        }

        AsyncData {
            overlapped: AsyncData::create_overlapped(),
            file_handle: file_handle,
            buffer: buffer,
            data_type: DataType::Read(ReadData {
                read_size: read_size,
                callback: callback,
            }),
        }
    }

	// -------------------------------------------------------------------------
    pub fn execute_error_callback(&self, error: String) {
    	match &self.data_type {
    		&DataType::Read(ref read_data) => read_data.callback.as_ref()(Err(error)), 
    		&DataType::Write(ref write_data) => write_data.callback.as_ref()(Err(error))
    	}
    }
    
    // -------------------------------------------------------------------------
    fn create_overlapped() -> OVERLAPPED {
        OVERLAPPED {
            OffsetHigh: 0,
            hEvent: null_mut(),
            Offset: 0,
            Internal: 0,
            InternalHigh: 0,
        }
    }
}