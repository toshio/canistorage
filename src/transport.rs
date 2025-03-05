use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write, ErrorKind};
use serde::{Serialize, Deserialize};
use candid::{CandidType, Principal};
use sha2::{Sha256, Digest};

const SUCCESS: u32 = 0;
const ERROR_FILE_NOT_FOUND: u32 = 1;
const ERROR_FILE_ALREADY_EXISTS: u32 = 2;
const ERROR_INVALID_PATH: u32 = 3;

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

fn validate_path(path:&String) -> Result<(), String> {
    // length
    if path.len() == 0 {
        return Err("Path is empty".to_string());
    }

    // starts with
    #[cfg(test)]
    if path.starts_with("./.test/") == false {
        return Err("Not full path".to_string());
    }
    #[cfg(not(test))]
    if path.starts_with("/") == false {
        return Err("Not full path".to_string());
    }

    // invalid characters
    if ["..", "`"].iter().any(|s| path.contains(s)) {
        return Err("Path contains invalid characters".to_string());
    }
    Ok(())
}

fn file_info_path(path:&String) -> String {
    match path.rfind("/") {
        Some(index) => {
            format!("{}`{}", &path[0..index +1], &path[index + 1..])
        },
        None => {
            format!("`{}", path)
        }
    }
}

fn get_file_info(path:&String) -> Option<FileInfo> {
    let file = File::open(file_info_path(path)).unwrap();
    let reader = BufReader::new(file);

    let result = serde_cbor::from_reader(reader).unwrap();
    Some(result)
}

fn set_file_info(path:&String, info:FileInfo) -> () {
    let _ = fs::write(file_info_path(path), serde_cbor::to_vec(&info).unwrap());
}

/// Uload a file to the canister (less than 2MiB)
#[ic_cdk::update]
fn save(path:String, mime_type:String, data:Vec<u8>, overwrite:bool) -> SaveResult {
    match validate_path(&path) {
        Err(e) => {
            return SaveResult {
                code: ERROR_INVALID_PATH,
                message: Some(e)
            }
        },
        _ => {}
    };
    match if overwrite == true {
        OpenOptions::new().write(true).create(true).truncate(true).open(&path)
    } else {
        OpenOptions::new().write(true).create_new(true).open(&path)
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
                    set_file_info(&path, info);

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

// FIXME result should be more detailed
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

// FIXME result should be more detailed
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

// FIXME result should be more detailed
#[ic_cdk::update(name="createDirectory")]
fn create_directory(path:String) -> bool {
    match validate_path(&path) {
        Err(e) => {
            return false;
        },
        _ => {}
    };
    match fs::create_dir(&path) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            false
        }
    }
}

// FIXME result should be more detailed
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
