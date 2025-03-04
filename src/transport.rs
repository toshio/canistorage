use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, ErrorKind};
use serde::{Serialize, Deserialize};
use candid::{CandidType, Principal};
use sha2::{Sha256, Digest};

const SUCCESS: u32 = 0;
const ERROR_FILE_NOT_FOUND: u32 = 1;
const ERROR_FILE_ALREADY_EXISTS: u32 = 2;
const ERROR_UNKNOWN: u32 = u32::MAX;

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
fn save(path:String, mime_type:String, data:Vec<u8>, overwrite:bool) -> SaveResult {
    match if overwrite == true {
        OpenOptions::new().write(true).create(true).truncate(true).open(path)
    } else {
        OpenOptions::new().write(true).create_new(true).open(path)
    } {
        Ok(mut file) => {
            match file.write_all(&data) {
                Ok(_) => {
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
                        code: SUCCESS,
                        message: None
                    }
                },
                Err(e) => match e.kind() {
                    _ => SaveResult {
                        code: ERROR_UNKNOWN,
                        message: Some("Failed to write data".to_string())
                    }
                }
            }
        },
        Err(e) => match e.kind() {
            ErrorKind::AlreadyExists => {
                SaveResult {
                    code: ERROR_FILE_ALREADY_EXISTS,
                    message: Some("File already exists".to_string())
                }
            },
            _ => {
                eprintln!("Error: {:?}", e);
                SaveResult {
                    code: ERROR_UNKNOWN,
                    message: Some(format!("{:?}", e))
                }
            }
        }
    }
}

#[ic_cdk::query]
fn load(path:String) -> LoadResult {
    // FIXME check file size before read to 
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            let size = file.read_to_end(&mut buffer);
            LoadResult {
                code: SUCCESS,
                data: Some(buffer),
                message: None,
            }
        },
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                LoadResult {
                    code: ERROR_FILE_NOT_FOUND,
                    data: None,
                    message: Some("File not found".to_string()),
                }
            },
            _ => {
                eprintln!("Error: {:?}", e);
                LoadResult {
                    code: ERROR_UNKNOWN,
                    data: None,
                    message: Some(format!("{:?}", e)),
                }
            }
        }
    }
}

#[ic_cdk::update(name="delete")]
fn delete(path:String) -> bool {
    match fs::remove_file(path) {
        Ok(_) => true,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                false
            },
            _=> {
                // FIXME: should be error
                false
            },
        }
    }
}

#[ic_cdk::query(name="listFiles")]
fn list_files(path:String) -> Vec<String> {
    let paths = fs::read_dir(path).unwrap();
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
    fn test_save() {
        let _context = setup();

        // new file
        let data = "Hello, World!".as_bytes().to_vec();
        save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), false);
        let result = load("./.test/file.txt".to_string());
        assert_eq!(result.code, SUCCESS);
        assert_eq!(result.data.unwrap(), data);

        // overwrite
        let data = "Hello, World!".as_bytes().to_vec();
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), true);
        assert_eq!(result.code, SUCCESS);
        let result = load("./.test/file.txt".to_string());
        assert_eq!(result.code, SUCCESS);
        assert_eq!(result.data.unwrap(), data);

        // error
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), false);
        // FIXME should be error.
        assert_eq!(result.code, ERROR_FILE_ALREADY_EXISTS);
    }

    #[test]
    fn test_delete() {
        let _context = setup();

        // new file
        let data = "Hello, World!".as_bytes().to_vec();
        save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), false);
        let result = load("./.test/file.txt".to_string());
        assert_eq!(result.code, SUCCESS);
        assert_eq!(result.data.unwrap(), data);

        // delete
        let result = delete("./.test/file.txt".to_string());
        assert_eq!(result, true);

        // delete (File not found)
        let result = delete("./.test/file.txt".to_string());
        assert_eq!(result, false);
    }
}
