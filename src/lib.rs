extern crate kernel32;
extern crate winapi;
extern crate time;

pub mod file;
mod win_api_helper;
mod async_data;
mod io_worker;
mod tools;


#[cfg(test)]
mod test {
    use file::File;
    use std::io::Write;
    use std::io::ErrorKind;
    use std;
	use io_worker::add_usize_to_u32_pair;
	
    // -----------------------------------------------------------------------------
    struct Notifier {
        pair: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    }

    impl Notifier {
        // -------------------------------------------------------------------------
        fn notify(&self) {
            let &(ref mutex, ref cond_var) = &*self.pair;
            let mut started = mutex.lock().unwrap();
            *started = true;
            cond_var.notify_one();
        }
    }

    // -----------------------------------------------------------------------------
    struct Waiter {
        pair: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
    }

    // -----------------------------------------------------------------------------
    impl Waiter {
        fn wait(&self) {
            let &(ref mutex, ref cond_var) = &*self.pair;
            let mut mutex_gard = mutex.lock().unwrap();
            let duration = std::time::Duration::new(3, 0);

            while !*mutex_gard {
                let (gard, timeout_result) = cond_var.wait_timeout(mutex_gard, duration).unwrap();
                mutex_gard = gard;
                if timeout_result.timed_out() {
                    panic!("Timeout!");
                }
            }
        }
    }
    // -----------------------------------------------------------------------------
    fn create_waiter() -> (Waiter, Notifier) {
        let pair = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));

        (Waiter { pair: pair.clone() }, Notifier { pair: pair })
    }

    // -----------------------------------------------------------------------------
    struct Test {
        path: &'static str,
    }

    // -----------------------------------------------------------------------------
    impl Test {
        // -------------------------------------------------------------------------
        fn new() -> Test {
            let test = Test { path: "test" };
            std::fs::remove_file(test.path).
            	or_else(|error| -> std::io::Result<()> { 
            		if error.kind() == ErrorKind::NotFound {
            			Ok(())
            		} else {
            			Err(error)
            		}
            	}).unwrap();
            test
        }

        // -------------------------------------------------------------------------
        fn create_file(&self, data: &[u8]) {
            let mut fs = std::fs::File::create(self.path).unwrap();

            fs.write_all(data).unwrap();
        }

        // -----------------------------------------------------------------------------
        fn create_data(data_size: usize) -> Vec<u8> {
            let mut data = Vec::new();

            let mut value: u8 = 0;
            for _ in 0..data_size {
                data.push(value);
                if value == u8::max_value() {
                	value = 0;
                } else {
                    value += 1;
                }
            }
            data
        }

        // -----------------------------------------------------------------------------
        fn test_read_all(&self, data_size: usize) {
            let (waiter, notifier) = create_waiter();
            let data = Test::create_data(data_size);

            self.create_file(&data);
            let mut file = File::open(self.path).unwrap();
            file.read_all(Box::new(move |data_result| {
                let read_data = data_result.unwrap();
                assert_eq!(data.len(), read_data.len());
                assert_eq!(data, read_data);
                notifier.notify();
            }));
            waiter.wait();
        }

        // -----------------------------------------------------------------------------
        fn check_read(&self, expected_data: Vec<u8>) {
            let (waiter, notifier) = create_waiter();
            let mut file = File::open(self.path).unwrap();
            file.read_all(Box::new(move |data_result| {
                let read_data = data_result.unwrap();
                assert_eq!(expected_data.len(), read_data.len());
                assert_eq!(expected_data, read_data);
                notifier.notify();
            }));
            waiter.wait();
        }

        // -----------------------------------------------------------------------------
        fn write_sync(&self, data: Vec<u8>) {
            let (waiter, notifier) = create_waiter();

            let mut file = File::create(self.path).unwrap();
            file.write_all(data,
                           Box::new(move |result| {
                               result.unwrap();
                               notifier.notify();
                           }));
            waiter.wait();
        }

        // -----------------------------------------------------------------------------
        fn test_write_all(&self, data_size: usize) {
            let data = Test::create_data(data_size);
            let expected_data = data.clone();
            self.write_sync(data);
            self.check_read(expected_data);
        }
    }

    // -----------------------------------------------------------------------------
    impl Drop for Test {
        fn drop(&mut self) {
            std::fs::remove_file(self.path).unwrap();
        }
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_create() {
        let test = Test::new();
        File::create(test.path).unwrap();
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_create_file_exist() {
        let test = Test::new();

        test.create_file(b"data");
        assert!(File::create(test.path).is_err());
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_open() {
        let test = Test::new();

        test.create_file(b"data");
        File::open(test.path).unwrap();
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_open_file_not_exist() {
        let test = Test::new();

        assert!(File::open(test.path).is_err());
        test.create_file(b"For Test::Drop");
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_read_all_small() {
        let test = Test::new();

        test.test_read_all(10);
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_read_all_cluster_factor() {
        let test = Test::new();

        test.test_read_all(2 * File::get_cluster_size());
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_read_big() {
        let test = Test::new();

        test.test_read_all(2 * File::get_cluster_size() + 3);
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_write_small() {
        let test = Test::new();

		test.test_write_all(42);
    }

    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_write_cluster_factor() {
        let test = Test::new();

        test.test_write_all(2 * File::get_cluster_size());
    }
    
    // -----------------------------------------------------------------------------
    #[test]
    fn it_test_add_usize_to_u32_pair() {
        
        assert_eq!((0x12345679, 0x3), add_usize_to_u32_pair(0x12345678, 0x1, 0x200000001 ));

    }
} 

// $$ TODO
// $$$ test special chars
// $$ Improve error handling
// $$ create a write buffer to be able to reuse the buffer
// $$ Implement the right logic for File::Drop
// $$ use rigth value for read_all