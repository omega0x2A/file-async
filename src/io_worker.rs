use tools::read_file_async_data_buffer;

use winapi::INVALID_HANDLE_VALUE;
use winapi::HANDLE;
use winapi::FILE_END;
use winapi::LARGE_INTEGER;

use std::ptr::null_mut;
use std::thread;

use std::mem::transmute;
use async_data::AsyncData;
use async_data::DataType;
use std::sync::Once;
use std::sync::ONCE_INIT;

use win_api_helper::get_queued_completion_status;
use win_api_helper::set_file_pointer_ex;
use win_api_helper::set_end_of_file;
use win_api_helper::create_io_completion_port;
use win_api_helper::get_system_info;

//-----------------------------------------------------------------------------
#[derive(Copy, Clone)]
struct WinHandle
{
	handle: HANDLE
}

unsafe impl Sync for WinHandle {}
unsafe impl Send for WinHandle {}

static INIT_COMPLETION_PORT: Once = ONCE_INIT;
static mut IO_COMPLETION_PORT: Option<Result<WinHandle, &'static str>> = None;


//-----------------------------------------------------------------------------
pub fn init_static_completion_port_once() -> Result<HANDLE, &'static str> {
	unsafe {
		INIT_COMPLETION_PORT.call_once(|| {
			IO_COMPLETION_PORT = Some(create_io_workers().
				map_err(|_| "Error in create_io_workers"));
		});
		IO_COMPLETION_PORT.unwrap().map(|completion_port| completion_port.handle)
	}
}

//-----------------------------------------------------------------------------
fn create_io_workers() -> Result<WinHandle, String> {
	let io_completion_port = try!(create_io_completion_port(
		INVALID_HANDLE_VALUE, null_mut(), 0, 0));
	let win_handle = WinHandle{ handle: io_completion_port};
	let system_info = get_system_info();
    let nb_workers = system_info.dwNumberOfProcessors * 2;
    for _ in 0..nb_workers {
    	println!("Start background thread");
	    thread::spawn(move || { wait_for_io(win_handle) });
	}
	Ok(win_handle)
}

//-----------------------------------------------------------------------------
fn wait_for_io(win_handle: WinHandle) {	
	loop {
		let completion_status = get_queued_completion_status(win_handle.handle)
										.expect("Fatal error ");
		unsafe {								
			let data: Box<AsyncData> = transmute(completion_status.overlapped);
			
			if completion_status.end_of_file {
				read_async(data, 0);
			} else {
				read_async(data, completion_status.nb_bytes_transferred as usize);
			}	
		}
	}	
}

//-----------------------------------------------------------------------------
fn execute_callback(async_data: &mut AsyncData, nb_bytes_transferred: usize) -> Option<usize> {
	match &mut async_data.data_type {
		&mut DataType::Read(ref mut read_data) => {
			let buffer = &mut async_data.buffer;		
			let buffer_size = buffer.len();
			let read_size = read_data.read_size;
					
			if nb_bytes_transferred < read_data.read_size {
				let new_size = buffer_size - (read_size - nb_bytes_transferred);
				
				buffer.resize(new_size, 0);
				read_data.callback.as_ref()(Ok(&buffer));
				None
			} else {
				Some(read_size)
			}
		},
		&mut DataType::Write(ref mut write_data) => {
			let file_size = write_data.bytes_to_write as isize - nb_bytes_transferred as isize;
			
			let res = set_file_pointer_ex(
					async_data.file_handle, 
					file_size as LARGE_INTEGER, 
					null_mut(),
					FILE_END).
				and(set_end_of_file(async_data.file_handle));
				
			write_data.callback.as_ref()(res);
			None
		}
	}	
}

//-----------------------------------------------------------------------------
pub fn add_usize_to_u32_pair(value: u32, value_high: u32, usize_value: usize ) -> (u32, u32) {
	let new_value = value as u64 + ((value_high as u64) << 32) + usize_value as u64;
	
	((new_value & 0xffffffff) as u32, (new_value >> 32) as u32)
}	

//-----------------------------------------------------------------------------
fn read_async(mut async_data: Box<AsyncData>, nb_bytes_transferred: usize ) {	
	match execute_callback(async_data.as_mut(), nb_bytes_transferred) {
		Some(next_read_size) => {
			let buffer_size = async_data.buffer.len();
			let new_buffer_size = buffer_size + next_read_size;
			
			async_data.buffer.reserve(new_buffer_size);
			unsafe {
				async_data.buffer.set_len(new_buffer_size);
			}
			
			let (offset, offset_high) = add_usize_to_u32_pair(
					async_data.overlapped.Offset,
					async_data.overlapped.OffsetHigh,
					next_read_size) ;
			
			async_data.overlapped.Offset = offset;
			async_data.overlapped.OffsetHigh = offset_high;
										
			unsafe {
				read_file_async_data_buffer(
						async_data.file_handle, 
						async_data.buffer.as_ptr().offset(buffer_size as isize), 
						next_read_size, 
						async_data);
			}
		}
		None => {}
	}					
}