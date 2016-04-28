use win_api_helper::write_file_async;
use win_api_helper::read_file_async;

use win_api_helper::AsyncOperationError;

use async_data::AsyncData;
use winapi::HANDLE;

//-----------------------------------------------------------------------------
pub fn write_file_async_data(file: HANDLE, async_data: Box<AsyncData>) {
	let result = write_file_async(file,
                             async_data.buffer.as_ptr(),
                             async_data.buffer.len(),
                             async_data);
    handle_async_operation_error(result);
}

//-----------------------------------------------------------------------------
pub fn read_file_async_data(file: HANDLE, async_data: Box<AsyncData>) {
	read_file_async_data_buffer(
							file,
                            async_data.buffer.as_ptr(),
                            async_data.buffer.len(),
                            async_data);
}

//-----------------------------------------------------------------------------
pub fn read_file_async_data_buffer(
		file: HANDLE, 
   		buffer: *const u8,
   		buffer_size: usize,
        async_data: Box<AsyncData>) {
	let result = read_file_async(file, buffer, buffer_size, async_data);
	handle_async_operation_error(result);
}
        
//-----------------------------------------------------------------------------
fn handle_async_operation_error(result: Result<(), AsyncOperationError<AsyncData>>) {
	match result {
 		Ok(_) => {},
 			Err(async_operation_error) => {
 				let async_data = async_operation_error.overlapped_box;
 				async_data.execute_error_callback(async_operation_error.error);
 			}
        }
}