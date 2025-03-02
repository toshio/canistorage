use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use candid::{CandidType, Principal};
use sha2::{Sha256, Digest};

#[derive(CandidType, Serialize, Deserialize)]
pub struct FileInfo {
    size: u64,
    created_at: u64,
    updated_at: u64,
    mime_type: String,
    sha256: [u8; 32],
    readable: Vec<Principal>,
    writable: Vec<Principal>,
    signature: Option<Vec<u8>>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct SaveOption {
    create: bool,
    truncate: bool,
    append: bool,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct SaveResult {
    code: u32,
    message: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct LoadResult {
    code: u32,
    data: Option<Vec<u8>>,
    message: Option<String>
}

fn get_file_info(path:String) -> Option<FileInfo> {
    // TODO: Implement
    None
}

fn set_file_info(path:String, info:FileInfo) -> () {
    // TODO: Implement
}

/// Uload a file to the canister (less than 2MiB)
#[ic_cdk::update]
fn save(path:String, mime_type:String, data:Vec<u8>, option:SaveOption) -> SaveResult {
    match OpenOptions::new().write(true).create(option.create).truncate(option.truncate).append(option.append).open(path) {
        Ok(mut file) => {
            file.write_all(&data).unwrap();
            let info = FileInfo {
                size: data.len() as u64,
                created_at: 0,
                updated_at: 0,
                mime_type: mime_type,
                sha256: Sha256::digest(data).into(),
                readable: Vec::new(),
                writable: Vec::new(),
                signature: None,
            };
            // TODO set_file_info()

            SaveResult {
                code: 0,
                message: None
            }
        },
        Err(e) => {
            SaveResult {
                code: 1,
                message: Some(format!("{:?}", e))
            }
        }
    }
}

#[ic_cdk::query]
fn load(path:String) -> LoadResult {
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            let _ = file.read_to_end(&mut buffer);
            LoadResult {
                code: 0,
                data: Some(buffer),
                message: None,
            }
        },
        Err(e) => {
            LoadResult {
                code: 1,
                data: None,
                message: Some(format!("{:?}", e)),
            }
        }
    }
}

#[ic_cdk::update(name="delete")]
fn delete(path:String) -> bool {
    match fs::remove_file(path) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            false
        }
    }
}

#[ic_cdk::query(name="listFiles")]
fn list_files() -> Vec<String> {
    let paths = fs::read_dir(".").unwrap();
    let mut files = Vec::new();
    for path in paths {
        let path = path.unwrap().path();
        let path = path.to_str().unwrap().to_string();
        files.push(path);
    }
    files
}

#[ic_cdk::update(name="createDirectory")]
fn create_directory(path:String) -> bool {
    match fs::create_dir(path) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            false
        }
    }
}

#[ic_cdk::update(name="removeDirectory")]
fn remove_directory(path:String) -> bool {
    match fs::remove_dir(path) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            false
        }
    }
}

// Test
#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext {
    }
    fn setup() -> TestContext {
        let _ = fs::remove_dir_all("./.test/");
        let _ = fs::create_dir("./.test/").unwrap();
        TestContext {
        }
    }
    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all("./.test/");
        }
    }

    #[test]
    fn test_save_new() {
        let _context = setup();

        // new file
        let data = "Hello, World!".as_bytes().to_vec();
        save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), SaveOption { create:true, truncate:false, append: false});
        let result = load("./.test/file.txt".to_string());
        assert_eq!(result.code, 0);
        assert_eq!(result.data.unwrap(), data);

        // truncate
        let data = "Hello, World!".as_bytes().to_vec();
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), SaveOption { create:true, truncate:true, append: false});
        assert_eq!(result.code, 0);
        let result = load("./.test/file.txt".to_string());
        assert_eq!(result.code, 0);
        assert_eq!(result.data.unwrap(), data);

        // append
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), SaveOption { create:true, truncate:false, append:true});
        assert_eq!(result.code, 0);
        let result = load("./.test/file.txt".to_string());
        assert_eq!(result.code, 0);
        assert_eq!(result.data.unwrap(), vec![data.clone(), data.clone()].concat());

        // error
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), SaveOption { create:false, truncate:false, append:false});
        // FIXME should be error.
        // assert_eq!(result.code, 1);
    }
}
