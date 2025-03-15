/// Canistorage
/// 
/// Copyright© 2025 toshio
///
use std::cell::RefCell;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write, ErrorKind};
use serde::{Serialize, Deserialize};
use candid::{CandidType, Principal};
use sha2::{Sha256, Digest};

const MIMETYPE_DIRECTORY: &str = "canistorage/directory";

const SUCCESS: u32 = 0;
const ERROR_FILE_NOT_FOUND: u32 = 1;
const ERROR_FILE_ALREADY_EXISTS: u32 = 2;
const ERROR_DIRECTORY_NOT_FOUND: u32 = 3;
const ERROR_DIRECTORY_ALREADY_EXISTS: u32 = 4;
const ERROR_INVALID_PATH: u32 = 5;
const ERROR_PERMISSION_DENIED: u32 = 6;
const ERROR_UNKNOWN: u32 = u32::MAX;

/////////////////////////////////////////////////////////////////////////////
// For Unit Test
/////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
const ROOT: &str = "./.test";

/// Returns the current time in milliseconds
#[cfg(test)]
fn time() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() as u64
}

#[cfg(test)]
thread_local! {
    static CALLER:RefCell<Principal> = RefCell::new(Principal::anonymous());
}

#[cfg(test)]
fn set_caller(principal:Principal) -> () {
    CALLER.with(|caller| {
        *caller.borrow_mut() = principal;
    })
}
#[cfg(test)]
fn caller() -> Principal {
    CALLER.with(|caller| {
        *caller.borrow()
    })
}

/////////////////////////////////////////////////////////////////////////////
// For Production
/////////////////////////////////////////////////////////////////////////////
#[cfg(not(test))]
const ROOT: &str = "/";

/// Returns the current time in milliseconds
#[cfg(not(test))]
fn time() -> u64 {
    ic_cdk::api::time() / 1_000_000 // milliseconds
}

#[cfg(not(test))]
fn caller() -> Principal {
    ic_cdk::api::caller()
}

/////////////////////////////////////////////////////////////////////////////
// Data Structures
/////////////////////////////////////////////////////////////////////////////
#[derive(CandidType, Serialize, Deserialize)]
pub struct Error {
    code:u32,
    message: String,
}
macro_rules! Error {
    ($code:expr, $message:expr) => {
        Err(Error {
            code: $code,
            message: $message.to_string(),
        })
    };
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct FileInfo {
    size: u64,  // bytes
    creator: Principal,
    created_at: u64, // milliseconds
    updater: Principal,
    updated_at: u64, // milliseconds
    mime_type: String,
    sha256: [u8; 32],
    manageable: Vec<Principal>, // Grant or Revoke permission
    readable: Vec<Principal>,
    writable: Vec<Principal>,
    signature: Option<Vec<u8>>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct AddPermissionResult {
    code: u32,
    message: Option<String>,
}
#[derive(CandidType, Serialize, Deserialize)]
pub struct RemovePermissionResult {
    code: u32,
    message: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct SaveResult {
    code: u32,
    message: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct LoadResult {
    code: u32,
    message: Option<String>,
    data: Option<Vec<u8>>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct DeleteResult {
    code: u32,
    message: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct CreateDirectoryResult {
    code: u32,
    message: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct DeleteDirectoryResult {
    code: u32,
    message: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct ListFilesResult {
    code: u32,
    message: Option<String>,
    data: Option<Vec<String>>,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct HasPermissionResult {
    code: u32,
    message: Option<String>,
    manageable: bool,
    writable: bool,
    readable: bool,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct Info {
    size: u64,  // bytes
    creator: Principal,
    created_at: u64, // milliseconds
    updater: Principal,
    updated_at: u64, // milliseconds
    mime_type: String,
    sha256: [u8; 32],
}


/////////////////////////////////////////////////////////////////////////////
// Functions
/////////////////////////////////////////////////////////////////////////////

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

/// returns file info path (metadata of file)
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
    // TODO Error handling
    let _ = fs::write(file_info_path(path), serde_cbor::to_vec(info).unwrap());
}

fn delete_file_info(path:&String) -> () {
    // TODO Error handling
    let _ = fs::remove_file(file_info_path(path));
}

// returns temporary path for saving a file
fn temp_path(path:&String) -> String {
    if path == "/" {
        return "/``".to_string();
    }
    match path.rfind("/") {
        Some(index) => {
            format!("{}``{}", &path[0..index +1], &path[index + 1..])
        },
        None => {
            // FIXME Not expected
            format!("``{}", path)
        }
    }
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

/// Returns whether the specified path is manageable or not
///
/// # Arguments
///
/// * `principal` - Principal to check
/// * `path` - must start with ROOT
/// * `file_info` - FileInfo
fn check_manage_permission(principal:&Principal, path:&String, file_info:Option<&FileInfo>) -> bool {
    // First, check manageable of file_info
    if let Some(info) = file_info {
        if info.manageable.iter().any(|p| p == principal) {
            // Found manageable
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
        check_manage_permission(principal, &parent_path, parent_info.as_ref())
    }
}

#[ic_cdk::update]
fn add_permission(principal:Principal, path:String, manageable:bool, readable:bool, writable:bool) -> AddPermissionResult {
    match validate_path(&path) {
        Err(e) => {
            return AddPermissionResult {
                code: ERROR_INVALID_PATH,
                message: Some(e)
            }
        },
        _ => {}
    };

    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_manage_permission(&caller, &path, file_info.as_ref()) {
        return AddPermissionResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string())
        };
    }

    // Check whether file exists or not
    match file_info {
        Some(mut new_info) => {
            if manageable {
                if new_info.manageable.binary_search_by_key(&&principal, |p|p).is_err() {
                    new_info.manageable.push(principal);
                    new_info.manageable.sort();
                }
            }
            if readable {
                if new_info.readable.binary_search_by_key(&&principal, |p|p).is_err() {
                    new_info.readable.push(principal);
                    new_info.readable.sort();
                }
            }
            if writable {
                if new_info.writable.binary_search_by_key(&&principal, |p|p).is_err() {
                    new_info.writable.push(principal);
                    new_info.writable.sort();
                }
            }
            set_file_info(&path, &new_info);

            AddPermissionResult {
                code: SUCCESS,
                message: None
            }
        },
        None => AddPermissionResult {
            code: ERROR_FILE_NOT_FOUND, // TODO File or directory
            message: Some("File not found".to_string())
        }
    }
}


#[ic_cdk::update]
fn remove_permission(principal:Principal, path:String, manageable:bool, readable:bool, writable:bool) -> RemovePermissionResult {
    match validate_path(&path) {
        Err(e) => {
            return RemovePermissionResult {
                code: ERROR_INVALID_PATH,
                message: Some(e)
            }
        },
        _ => {}
    };

    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_manage_permission(&caller, &path, file_info.as_ref()) {
        return RemovePermissionResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string())
        };
    }

    // Check whether file exists or not
    match file_info {
        Some(mut new_info) => {
            if manageable {
                match new_info.manageable.binary_search_by_key(&&principal, |p|p) {
                    Ok(index) => {
                        new_info.manageable.remove(index);
                    },
                    Err(_) =>{}
                }
            }
            if readable {
                match new_info.readable.binary_search_by_key(&&principal, |p|p) {
                    Ok(index) => {
                        new_info.readable.remove(index);
                    },
                    Err(_) =>{}
                }
            }
            if writable {
                match new_info.writable.binary_search_by_key(&&principal, |p|p) {
                    Ok(index) => {
                        new_info.writable.remove(index);
                    },
                    Err(_) =>{}
                }
            }
            set_file_info(&path, &new_info);

            RemovePermissionResult {
                code: SUCCESS,
                message: None
            }
        },
        None => RemovePermissionResult {
            code: ERROR_FILE_NOT_FOUND, // TODO File or directory
            message: Some("File not found".to_string())
        }
    }
}

/// Uload a file to the canister (less than 2MiB)
#[ic_cdk::update]
fn save(path:String, mime_type:String, data:Vec<u8>, overwrite:bool) -> SaveResult {
    // First, check path
    match validate_path(&path) {
        Err(e) => {
            return SaveResult {
                code: ERROR_INVALID_PATH,
                message: Some(e)
            }
        },
        _ => {}
    };

    // Second, check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return SaveResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string())
        };
    }

    // Third, check whether file exists or not
    if file_info.is_some() && overwrite == false {
        return SaveResult {
            code: ERROR_FILE_ALREADY_EXISTS, // TODO File or directory
            message: Some("File already exists".to_string())
        }
    }

    // TODO save as temp, and then rename it
    let temp_path = temp_path(&path);

    let file = OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path);
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
                                creator: caller,
                                created_at: now,
                                updater: caller,
                                updated_at: now,
                                mime_type: mime_type,
                                sha256: Sha256::digest(data).into(),
                                manageable: Vec::new(),
                                readable: Vec::new(),
                                writable: Vec::new(),
                                signature: None,
                            }
                        }
                    };

                    match fs::rename(&temp_path, &path) {
                        Ok(_) => {
                            set_file_info(&path, &info);
                            SaveResult {
                                code: SUCCESS,
                                message: None
                            }
                        },
                        Err(e) => SaveResult {
                            code: ERROR_UNKNOWN,
                            message: Some(format!("{:?}", e))
                        }
                    }
                },
                Err(e) => match e.kind() {
                    _ => SaveResult {
                        code: ERROR_UNKNOWN,
                        message: Some(format!("{:?}", e))
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
            SaveResult {
                code: ERROR_UNKNOWN,
                message: Some(format!("{:?}", e))
            }
        }
    }
}

#[ic_cdk::query]
fn load(path:String) -> LoadResult {
    // First, check path 
    match validate_path(&path) {
        Err(e) => {
            return LoadResult {
                code: ERROR_INVALID_PATH,
                message: Some(e),
                data: None
            }
        },
        _ => {}
    };

    // Second, check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return LoadResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string()),
            data: None
        };
    }

    // Third, check whether file exists or not
    if file_info.is_none() {
        return LoadResult {
            code: ERROR_FILE_NOT_FOUND,
            message: Some("File not found".to_string()),
            data: None
        }
    }

    // FIXME check file size before read to 
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            let size = file.read_to_end(&mut buffer);
            LoadResult {
                code: SUCCESS,
                message: None,
                data: Some(buffer)
            }
        },
        Err(e) => match e.kind() { // Not expected
            ErrorKind::NotFound => {
                LoadResult {
                    code: ERROR_FILE_NOT_FOUND,
                    message: Some("File not found".to_string()),
                    data: None
                }
            },
            _ => {
                eprintln!("Error: {:?}", e);
                LoadResult {
                    code: ERROR_UNKNOWN,
                    message: Some(format!("{:?}", e)),
                    data: None
                }
            }
        }
    }
}

// FIXME result should be more detailed
#[ic_cdk::update(name="delete")]
fn delete(path:String) -> DeleteResult {
    // First, check path 
    match validate_path(&path) {
        Err(e) => {
            return DeleteResult {
                code: ERROR_INVALID_PATH,
                message: Some(e)
            }
        },
        _ => {}
    };

    // Second, check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return DeleteResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string())
        };
    }

    match fs::remove_file(&path) {
        Ok(_) => {
            delete_file_info(&path);

            DeleteResult {
                code: SUCCESS,
                message: None
            }
        },
        Err(e) => match e.kind() {   
            ErrorKind::NotFound => DeleteResult {
                code: ERROR_FILE_NOT_FOUND,
                message: Some("File not found".to_string()),
            },
            _=> DeleteResult {
                code: ERROR_UNKNOWN,
                message: Some(format!("{:?}", e)),
            },
        }
    }
}

// FIXME result should be more detailed
#[ic_cdk::query(name="listFiles")]
fn list_files(path:String) -> ListFilesResult {
    match validate_path(&path) {
        Err(e) => {
            return ListFilesResult {
                code: ERROR_INVALID_PATH,
                message: Some(e),
                data: None
            }
        },
        _ => {}
    };

    let file_info = get_file_info(&path);
    let caller = caller();
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return ListFilesResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string()),
            data: None
        }
    }

    if file_info.is_none() {
        return ListFilesResult {
            code: ERROR_DIRECTORY_NOT_FOUND,
            message: Some("Directory not found".to_string()),
            data: None
        }
    }

    let entries = fs::read_dir(path).unwrap();
    let mut files:Vec<String> = entries
        .map(| entry | {
            let entry = entry.unwrap();
            let file_name = entry.path().file_name().unwrap().to_string_lossy().into_owned();
            if entry.file_type().unwrap().is_dir() { 
                format!("{}/", file_name)
            } else {
                file_name.to_string()
            }
        })
        .filter(| file | !file.starts_with("`")) // Remove file_info
        .collect();
    files.sort();
    ListFilesResult {
        code: SUCCESS,
        message: None,
        data: Some(files)
    }
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

    // Check write permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return CreateDirectoryResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string())
        }
    }

    if file_info.is_some() {
        return CreateDirectoryResult {
            code: ERROR_FILE_ALREADY_EXISTS,  // FIXME Dir or file exists
            message: Some("Directory already exists".to_string())
        }
    }

    match fs::create_dir(&path) {
        Ok(_) => {
            // create file_info
            set_file_info(&path, &FileInfo {
                size: 0,
                creator: caller,
                created_at: time(),
                updater: caller,
                updated_at: time(),
                mime_type: MIMETYPE_DIRECTORY.to_string(),
                sha256: [0; 32],
                manageable: Vec::new(),
                readable: Vec::new(),
                writable: Vec::new(),
                signature: None,
            });
            
            CreateDirectoryResult {
                code: SUCCESS,
                message: None
            }
        },
        Err(e) => CreateDirectoryResult {
            code: ERROR_UNKNOWN,
            message: Some(format!("{:?}", e))
        }
    }
}

// FIXME result should be more detailed
#[ic_cdk::update(name="deleteDirectory")]
fn delete_directory(path:String) -> DeleteDirectoryResult {
    match validate_path(&path) {
        Err(e) => {
            return DeleteDirectoryResult {
                code: ERROR_INVALID_PATH,
                message: Some(e),
            }
        },
        _ => {}
    };

    let file_info = get_file_info(&path);
    let caller = caller();
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return DeleteDirectoryResult {
            code: ERROR_PERMISSION_DENIED,
            message: Some("Permission denied".to_string()),
        }
    }

    if file_info.is_none() {
        return DeleteDirectoryResult {
            code: ERROR_DIRECTORY_NOT_FOUND,
            message: Some("Directory not found".to_string()),
        }
    }

    match fs::remove_dir(&path) {
        Ok(_) => {
            delete_file_info(&path);
            DeleteDirectoryResult {
                code: SUCCESS,
                message: None
            }
        },
        Err(e) => DeleteDirectoryResult {
            code: ERROR_UNKNOWN,
            message: Some(format!("{:?}", e)),
        }
    }
}

#[ic_cdk::query(name="getInfo")]
fn get_info(path:String) -> Result<Info,Error> {
    match validate_path(&path) {
        Err(e) => {
            return Error!(ERROR_INVALID_PATH, format!("{:?}", e));
        },
        _ => {}
    };

    let file_info = get_file_info(&path);
    let caller = caller();
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return Error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    match file_info {
        Some(info) => Ok(Info {
            size: info.size,
            creator: info.creator,
            created_at: info.created_at,
            updater: info.updater,
            updated_at: info.updated_at,
            mime_type: info.mime_type,
            sha256: info.sha256
        }),
        None => Error!(ERROR_FILE_NOT_FOUND, "File not found")
    }
}


#[ic_cdk::query(name="hasPermission")]
fn has_permission(path:String) -> HasPermissionResult {
    match validate_path(&path) {
        Err(e) => {
            return HasPermissionResult {
                code: ERROR_INVALID_PATH,
                message: Some(e),
                manageable: false,
                readable: false,
                writable: false,
            }
        },
        _ => {}
    };

    let file_info = get_file_info(&path);
    let caller = caller();

    // TODO optimize algorithm
    HasPermissionResult {
        code: SUCCESS,
        message: None,
        manageable: check_manage_permission(&caller, &path, file_info.as_ref()),
        readable: check_read_permission(&caller, &path, file_info.as_ref()),
        writable: check_write_permission(&caller, &path, file_info.as_ref()),
    }
}

pub fn init() {
    let owner = caller();
    let now = time();
    ic_cdk::print(format!("Root Permission to {}", owner));
    set_file_info(&ROOT.to_string(), &FileInfo {
        size: 0,
        creator: owner,
        created_at: now,
        updater: owner,
        updated_at: now,
        mime_type: MIMETYPE_DIRECTORY.to_string(),
        sha256: [0; 32],
        manageable: vec![owner],
        readable: vec![owner],
        writable: vec![owner],
        signature: None,
    });    
}

/////////////////////////////////////////////////////////////////////////////
// Unit Test
/////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext {
    }
    fn setup() -> TestContext {
        let _ = fs::remove_dir_all(format!("{}/", ROOT)); // Root is "./.test/" for unit test
        let _ = fs::remove_file(file_info_path(&ROOT.to_string()));
        let _ = fs::create_dir(format!("{}/", ROOT));
        set_file_info(&ROOT.to_string(), &FileInfo {
            size: 0,
            creator: caller(),
            created_at: 0,
            updater: caller(),
            updated_at: 0,
            mime_type: MIMETYPE_DIRECTORY.to_string(),
            sha256: [0; 32],
            manageable: vec![caller()],
            readable: vec![caller()],
            writable: vec![caller()],
            signature: None,
        });
        TestContext {
        }
    }
    impl Drop for TestContext {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(format!("{}/", ROOT));
            let _ = fs::remove_file(file_info_path(&ROOT.to_string()));
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
        assert_eq!(result.code, SUCCESS);

        // delete (File not found)
        let result = delete("./.test/file.txt".to_string());
        assert_eq!(result.code, ERROR_FILE_NOT_FOUND);
    }

    #[test]
    fn test_file_info() {
        let _context = setup();

        // Root
        let principal_readable = Principal::from_text("f3umm-tovgf-tf7o6-o3oqc-iqlir-f6ufh-3lvrh-5wlic-6dmnu-gg4q7-6ae").unwrap(); // abandon x 12
        let principal_writable = Principal::from_text("ymtnq-243kz-shxxs-lfs7t-ihqhn-fntsv-wxvf3-kefpu-27hyr-wdczf-2ae").unwrap(); // ability x 12
        let file_info = FileInfo {
            size: 0,
            creator: caller(),
            created_at: 0,
            updater: caller(),
            updated_at: 0,
            mime_type: "".to_string(),
            sha256: [0; 32],
            manageable: Vec::new(),
            readable: vec![principal_readable.clone()],
            writable: vec![principal_writable.clone()],
            signature: None,
        };

        // Check of root
        let path = ROOT.to_string();
        set_file_info(&path, &file_info);
        assert_eq!(check_read_permission(&principal_readable, &path, Some(&file_info)), true);
        assert_eq!(check_read_permission(&principal_writable, &path, Some(&file_info)), false);
        assert_eq!(check_write_permission(&principal_readable, &path, Some(&file_info)), false);
        assert_eq!(check_write_permission(&principal_writable, &path, Some(&file_info)), true);

        // Check children (no permission found; check parent)
        let path = format!("{}/child", ROOT);
        assert_eq!(check_read_permission(&principal_readable, &path, None), true);
        assert_eq!(check_read_permission(&principal_writable, &path, None), false);
        assert_eq!(check_write_permission(&principal_readable, &path, None), false);
        assert_eq!(check_write_permission(&principal_writable, &path, None), true);

        // Check children (has permision)
        let principal_child_only = Principal::from_text("xm4xy-wgdl4-jhtba-hmdt7-kocg2-y47gj-wuwwg-oqbva-tydcp-6bvxn-7qe").unwrap(); // child x 12
        let file_info = FileInfo {
            size: 0,
            creator: caller(),
            created_at: 0,
            updater: caller(),
            updated_at: 0,
            mime_type: "".to_string(),
            sha256: [0; 32],
            manageable: Vec::new(),
            readable: vec![principal_child_only.clone()],
            writable: vec![principal_child_only.clone()],
            signature: None,
        };
        set_file_info(&path, &file_info);
        assert_eq!(check_read_permission(&principal_child_only, &path, Some(&file_info)), true);
        assert_eq!(check_write_permission(&principal_child_only, &path, Some(&file_info)), true);
        // hasPermission because of parent (Inherited)
        assert_eq!(check_read_permission(&principal_readable, &path, Some(&file_info)), true);
        assert_eq!(check_write_permission(&principal_writable, &path, Some(&file_info)), true);
        // No permission
        assert_eq!(check_read_permission(&principal_writable, &path, Some(&file_info)), false);
        assert_eq!(check_write_permission(&principal_readable, &path, Some(&file_info)), false);
    }

    #[test]
    fn test_list_files() {
        let _context = setup();

    }

    #[test]
    fn test_add_permission() {
        let _context = setup();
    }

    #[test]
    fn test_remove_permission() {
        let _context = setup();
    }

}
