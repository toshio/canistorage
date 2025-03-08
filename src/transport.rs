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

#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
const ROOT: &str = "./.test";
#[cfg(not(test))]
const ROOT: &str = "/";

#[derive(CandidType, Serialize, Deserialize)]
pub struct FileInfo {
    size: u64,  // bytes
    created_at: u64, // milliseconds
    updated_at: u64, // milliseconds
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

#[derive(CandidType, Serialize, Deserialize)]
pub struct CreateDirectoryResult {
    code: u32,
    message: Option<String>,
}

/// Returns the current time in milliseconds
#[cfg(test)]
fn time() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64
}

/// Returns the current time in milliseconds
#[cfg(not(test))]
fn time() -> u64 {
    ic_cdk::api::time() / 1_000_000 // milliseconds
}

/// validates the specified path
///
/// # Arguments
///
/// * `path` - path to check
/// 
fn validate_path(path:&String) -> Result<(), String> {
    // length
    if path.len() == 0 {
        return Err("Path is empty".to_string());
    }

    // starts with
    if path.starts_with(ROOT) == false {
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
    if path == "/" {
        return "/`".to_string();
    }
    match path.rfind("/") {
        Some(index) => {
            format!("{}`{}", &path[0..index +1], &path[index + 1..])
        },
        None => {
            // FIXME Not expected
            format!("`{}", path)
        }
    }
}

fn get_file_info(path:&String) -> Option<FileInfo> {
    match File::open(file_info_path(path)) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let result = serde_cbor::from_reader(reader).unwrap();
            Some(result)
       },
        Err(_) => {
            None
        }
    }
}

fn set_file_info(path:&String, info:&FileInfo) -> () {
    let _ = fs::write(file_info_path(path), serde_cbor::to_vec(info).unwrap());
}

/// Returns whether the specified path is readable or not
///
/// # Arguments
///
/// * `principal` - Principal to check
/// * `path` - must start with ROOT
/// * `file_info` - FileInfo
fn check_read_permission(principal:&Principal, path:&String, file_info:Option<&FileInfo>) -> bool {
    // First, check readable of file_info
    if let Some(info) = file_info {
        if info.readable.iter().any(|p| p == principal) {
            // Found readable
            return true;
        }
    }
    if path == ROOT {
        // Second, check if ROOT
        false
    } else {
        // Then, check parent file_info recursively
        let parent_path = match path.rfind("/") {
            Some(index) => {
                path[0..index].to_string()
            },
            None => {
                // Special case: "" -> "/""
                "/".to_string()
            }
        };
        let parent_info = get_file_info(&parent_path);
        check_read_permission(principal, &parent_path, parent_info.as_ref())
    }
}

/// Returns whether the specified path is writable or not
///
/// # Arguments
///
/// * `principal` - Principal to check
/// * `path` - must start with ROOT
/// * `file_info` - FileInfo
fn check_write_permission(principal:&Principal, path:&String, file_info:Option<&FileInfo>) -> bool {
    // First, check writeable of file_info
    if let Some(info) = file_info {
        if info.writable.iter().any(|p| p == principal) {
            // Found writeable
            return true;
        }
    }
    if path == ROOT {
        // Second, check if ROOT
        false
    } else {
        // Then, check parent file_info recursively
        let parent_path = match path.rfind("/") {
            Some(index) => {
                path[0..index].to_string()
            },
            None => {
                // Special case: "" -> "/""
                "/".to_string()
            }
        };
        let parent_info = get_file_info(&parent_path);
        check_write_permission(principal, &parent_path, parent_info.as_ref())
    }
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

    // Check whether file exists or not
    let file_info = get_file_info(&path);
    if file_info.is_some() && overwrite == false {
        return SaveResult {
            code: ERROR_FILE_ALREADY_EXISTS,
            message: Some("File not found".to_string())
        }
    }
    let file = OpenOptions::new().write(true).create(true).truncate(true).open(&path);
    match file {
        Ok(mut file) => {
            match file.write_all(&data) {
                Ok(_) => {
                    let info = match file_info {
                        Some(mut info) => {
                            // Update
                            info.size = data.len() as u64;
                            info.updated_at = time();
                            info.mime_type = mime_type;
                            info.sha256 = Sha256::digest(data).into();
                            info
                        },
                        None => {
                            // New
                            let now = time();
                            FileInfo {
                                size: data.len() as u64,
                                created_at: now,
                                updated_at: now,
                                mime_type: mime_type,
                                sha256: Sha256::digest(data).into(),
                                readable: Vec::new(),
                                writable: Vec::new(),
                                signature: None,
                            }
                        }
                    };
                    set_file_info(&path, &info);

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
fn create_directory(path:String) -> CreateDirectoryResult {
    match validate_path(&path) {
        Err(e) => {
            return CreateDirectoryResult {
                code: ERROR_INVALID_PATH,
                message: Some(e)
            }
        },
        _ => {}
    };

    let file_info = get_file_info(&path);
    if file_info.is_some() {
        return CreateDirectoryResult {
            code: ERROR_FILE_ALREADY_EXISTS,  // FIXME Dir or file exists
            message: Some("Directory already exists".to_string())
        }
    }
    match fs::create_dir(&path) {
        Ok(_) => CreateDirectoryResult {
            code: SUCCESS,
            message: None
        },
        Err(e) => CreateDirectoryResult {
            code: ERROR_UNKNOWN,
            message: Some(format!("{:?}", e))
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
        let _ = fs::remove_dir_all(format!("{}/", ROOT)); // Root is "./.test/" for unit test
        let _ = fs::remove_file(file_info_path(&ROOT.to_string()));
        let _ = fs::create_dir(format!("{}/", ROOT));
        TestContext {
        }
    }
    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(format!("{}/", ROOT));
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

    #[test]
    fn test_file_info() {
        let _context = setup();

        // Root
        let principalReadable = Principal::from_text("f3umm-tovgf-tf7o6-o3oqc-iqlir-f6ufh-3lvrh-5wlic-6dmnu-gg4q7-6ae").unwrap(); // abandon x 12
        let principalWritable = Principal::from_text("ymtnq-243kz-shxxs-lfs7t-ihqhn-fntsv-wxvf3-kefpu-27hyr-wdczf-2ae").unwrap(); // ability x 12
        let file_info = FileInfo {
            size: 0,
            created_at: 0,
            updated_at: 0,
            mime_type: "".to_string(),
            sha256: [0; 32],
            readable: vec![principalReadable.clone()],
            writable: vec![principalWritable.clone()],
            signature: None,
        };

        // Check of root
        let path = ROOT.to_string();
        set_file_info(&path, &file_info);
        assert_eq!(check_read_permission(&principalReadable, &path, Some(&file_info)), true);
        assert_eq!(check_read_permission(&principalWritable, &path, Some(&file_info)), false);
        assert_eq!(check_write_permission(&principalReadable, &path, Some(&file_info)), false);
        assert_eq!(check_write_permission(&principalWritable, &path, Some(&file_info)), true);

        // Check children (no permission found; check parent)
        let path = format!("{}/child", ROOT);
        assert_eq!(check_read_permission(&principalReadable, &path, None), true);
        assert_eq!(check_read_permission(&principalWritable, &path, None), false);
        assert_eq!(check_write_permission(&principalReadable, &path, None), false);
        assert_eq!(check_write_permission(&principalWritable, &path, None), true);

        // Check children (has permision)
        let principalChildOnly = Principal::from_text("xm4xy-wgdl4-jhtba-hmdt7-kocg2-y47gj-wuwwg-oqbva-tydcp-6bvxn-7qe").unwrap(); // child x 12
        let file_info = FileInfo {
            size: 0,
            created_at: 0,
            updated_at: 0,
            mime_type: "".to_string(),
            sha256: [0; 32],
            readable: vec![principalChildOnly.clone()],
            writable: vec![principalChildOnly.clone()],
            signature: None,
        };
        set_file_info(&path, &file_info);
        assert_eq!(check_read_permission(&principalChildOnly, &path, Some(&file_info)), true);
        assert_eq!(check_write_permission(&principalChildOnly, &path, Some(&file_info)), true);
        // hasPermission because of parent (Inherited)
        assert_eq!(check_read_permission(&principalReadable, &path, Some(&file_info)), true);
        assert_eq!(check_write_permission(&principalWritable, &path, Some(&file_info)), true);
        // No permission
        assert_eq!(check_read_permission(&principalWritable, &path, Some(&file_info)), false);
        assert_eq!(check_write_permission(&principalReadable, &path, Some(&file_info)), false);
    }
}
