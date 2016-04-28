use winapi::HANDLE;
use winapi::DWORD;
use winapi::FILE_ATTRIBUTE_NORMAL;
use winapi::FILE_FLAG_OVERLAPPED;
use winapi::INVALID_HANDLE_VALUE;
use winapi::LPOVERLAPPED;
use winapi::LPCVOID;
use winapi::LPVOID;
use winapi::ERROR_IO_PENDING;
use winapi::TRUE;
use winapi::FILE_FLAG_NO_BUFFERING;
use winapi::FORMAT_MESSAGE_FROM_SYSTEM;
use winapi::FORMAT_MESSAGE_ALLOCATE_BUFFER;
use winapi::FORMAT_MESSAGE_IGNORE_INSERTS;
use winapi::OPEN_EXISTING;
use winapi::CREATE_NEW;
use winapi::LPWSTR;
use winapi::BOOL;
use winapi::ULONG_PTR;
use winapi::INFINITE; 
use winapi::ERROR_HANDLE_EOF;
use winapi::SYSTEM_INFO;
use winapi::LARGE_INTEGER;
use winapi::PLARGE_INTEGER;

use kernel32::GetQueuedCompletionStatus;
use kernel32::CreateFileW;
use kernel32::WriteFile;
use kernel32::ReadFile;
use kernel32::GetLastError;
use kernel32::CreateIoCompletionPort;
use kernel32::FormatMessageW;
use kernel32::LocalFree;
use kernel32::SetFilePointerEx;
use kernel32::SetEndOfFile;
use kernel32::GetSystemInfo;

use std::path::Path;
use std::ptr::null_mut;
use std::ptr::null;
use std::mem::transmute;

// -----------------------------------------------------------------------------
pub fn create_file_async<P: AsRef<Path>>(path: P,
                                         desired_access: DWORD,
                                         creation_disposition: DWORD)
                                         -> Result<HANDLE, String> {
    unsafe {
        let path_str = path.as_ref().to_string_lossy().to_string(); 
		let filename = string_to_utf16(&path_str);

        let file = CreateFileW(filename,
                               desired_access,
                               0,
                               null_mut(),
                               creation_disposition,
                               FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED |
                               FILE_FLAG_NO_BUFFERING,
                               null_mut());
        if file == INVALID_HANDLE_VALUE {
            Err(get_create_file_async_error(&path_str, GetLastError(), creation_disposition))
        } else {
            Ok(file)
        }
    }
}

// -----------------------------------------------------------------------------
pub struct AsyncOperationError<T> {
	pub error: String,
	pub overlapped_box: Box<T>
}

// -----------------------------------------------------------------------------
pub fn write_file_async<T>(file: HANDLE,
                        buffer: *const u8,
                        buffer_size: usize,
                        overlapped_box: Box<T>)
                        -> Result<(), AsyncOperationError<T>> {
    let mut bytes_written: DWORD = 0;

    unsafe {
    	println!("Before WriteFile");

    	let overlapped = transmute::<Box<T>, LPOVERLAPPED>(overlapped_box);
        let status = WriteFile(file,
                     buffer as LPCVOID,
                     buffer_size as DWORD,
                     &mut bytes_written,
                     overlapped);
       check_async_operation::<T>(overlapped, status, "write", GetLastError())
    }
}


// -----------------------------------------------------------------------------
pub fn read_file_async<T>(file: HANDLE,
                       buffer: *const u8,
                       buffer_size: usize,
                       overlapped_box: Box<T>)
                       -> Result<(), AsyncOperationError<T>> {
    let mut bytes_read: DWORD = 0;

    unsafe {
    	let overlapped = transmute::<_, LPOVERLAPPED>(overlapped_box);
        let status = ReadFile(file,
                    buffer as LPVOID,
                    buffer_size as DWORD,
                    &mut bytes_read,
                    overlapped);
		check_async_operation::<T>(overlapped, status, "read", GetLastError())
    }
}

// -----------------------------------------------------------------------------
pub fn create_io_completion_port(file_handle: HANDLE,
                                 existing_completion_port: HANDLE,
                                 completion_key: ULONG_PTR,
                                 number_of_concurrent_threads: usize)
                                 -> Result<HANDLE, String> {
    unsafe {
        let handle = CreateIoCompletionPort(file_handle,
                                  existing_completion_port,
                                  completion_key,
                                  number_of_concurrent_threads as DWORD);
        if handle == null_mut() {
            Err(get_error_message("Error in CreateIoCompletion", GetLastError()))
        } else {
            Ok(handle)
        }
    }
}

// -----------------------------------------------------------------------------
pub struct CompletionStatus
{
	pub nb_bytes_transferred: DWORD, 
	pub overlapped: LPOVERLAPPED,
	pub end_of_file: bool
}

// -----------------------------------------------------------------------------
pub fn get_queued_completion_status(handle: HANDLE) -> Result<CompletionStatus, String> {
	let mut completion_key: ULONG_PTR = 0;
	let mut completion_status = CompletionStatus { nb_bytes_transferred: 0, 
		overlapped: null_mut(), end_of_file: false}; 
	
	unsafe {	
		if GetQueuedCompletionStatus(
		    		handle,
		    		&mut completion_status.nb_bytes_transferred, 
		    		&mut completion_key, 
		    		&mut completion_status.overlapped, 
		    		INFINITE) != TRUE {
				let error_id = GetLastError();
				
				if error_id == ERROR_HANDLE_EOF {
					completion_status.end_of_file = true;
				} 
				else
				{
					return Err(get_error_message("GetQueuedCompletionStatus", error_id));
				}
		}
	}
	Ok(completion_status)
}

// -----------------------------------------------------------------------------
pub fn set_file_pointer_ex(
	file: HANDLE,
    distance_to_move: LARGE_INTEGER,
    new_file_pointer: PLARGE_INTEGER,
    move_method: DWORD) -> Result<(), String>
{
	unsafe {
		if SetFilePointerEx(
					file,
				    distance_to_move,
				    new_file_pointer,
				    move_method) == 0 {
			Err(get_error_message("SetFilePointer", GetLastError()))
		} else {    
			Ok(())
		}
	}
}

// -----------------------------------------------------------------------------
pub fn set_end_of_file(file: HANDLE) -> Result<(), String> {
	unsafe {
		if SetEndOfFile(file) == 0 {
			Err(get_error_message("SetEndOfFile", GetLastError()))
		} else {    
			Ok(())
		}
	}
}

// -----------------------------------------------------------------------------
pub fn get_system_info() -> SYSTEM_INFO {
	let mut system_info = SYSTEM_INFO{
					wProcessorArchitecture: 0,
      				wReserved: 0,
					dwPageSize: 0,
					lpMinimumApplicationAddress: null_mut(),
					lpMaximumApplicationAddress: null_mut(),
					dwActiveProcessorMask: 0,
					dwNumberOfProcessors: 0,
					dwProcessorType: 0,
					dwAllocationGranularity: 0,
					wProcessorLevel: 0,
					wProcessorRevision: 0};
	
	unsafe {
		GetSystemInfo(&mut system_info);
	}
	system_info
}

// -----------------------------------------------------------------------------
fn string_to_utf16(str: &String) -> *const u16 {
	let mut buffer: Vec<u16> = str.encode_utf16().collect();
	buffer.push(0);
	buffer.as_ptr()
}

// -----------------------------------------------------------------------------
fn utf16_tostring(ptr: *mut u16) -> String {
	unsafe {
		let mut buffer = Vec::new();
		let mut length: isize = 0;
		while *ptr.offset(length) != 0 {
			buffer.push(*ptr.offset(length));
			length += 1;
		}
		String::from_utf16_lossy(&buffer)
	}
}

// -----------------------------------------------------------------------------
fn check_async_operation<T>(
	overlapped: LPOVERLAPPED,
	status: BOOL,
	operation_type: &str,
	error_id: DWORD) -> Result<(), AsyncOperationError<T>> {
	if status != TRUE && error_id == ERROR_IO_PENDING {
		Ok(())
	} else {
	    let error = if status == TRUE {
			format!("Error {} is synchronous", operation_type)            
        } else {
            get_error_message(
            		&format!(
            			"Error when performaing {} async", 
            			operation_type), error_id)
        };
        
        unsafe {
			Err(AsyncOperationError{ 
				error: error, 
				overlapped_box: transmute::<_, Box<T>>(overlapped)
			})
        }
    }
}

// -----------------------------------------------------------------------------
fn get_create_file_async_error(path_str: &str,
                               error_id: DWORD,
                               creation_disposition: DWORD) -> String {
    let message = match creation_disposition {
        d if d == OPEN_EXISTING => "cannot open ",
        d if d == CREATE_NEW => "cannot create ",
        _ => panic!("Invalid creation_disposition"),
    };
    let full_message = message.to_string() + path_str;

    get_error_message(&full_message, error_id)
}

// -----------------------------------------------------------------------------
fn get_error_message(context: &str, error_id: DWORD) -> String {
    format!("Error {}: ({}) {}.",
            context,
            error_id,
            get_last_error_message(error_id).unwrap_or("no message available".to_string()))
}

// -----------------------------------------------------------------------------
fn get_last_error_message(error_id: DWORD) -> Option<String> {
    unsafe {
        let mut message: LPWSTR = null_mut();
        if FormatMessageW(FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_ALLOCATE_BUFFER |
                          FORMAT_MESSAGE_IGNORE_INSERTS,
                          null(),
                          error_id,
                          0,
                          transmute(&message),
                          0,
                          null_mut()) == 0 {
            message = null_mut();
        }

        if message == null_mut() {
            None
        } else {
        	let mut message_str = utf16_tostring(message);
            if LocalFree(message as LPVOID) != null_mut() {
                message_str = message_str + ". In addition there is an error in LocalFree";
            }
            Some(message_str)
        }
    }
}
