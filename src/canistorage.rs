/// Canistorage
/// 
/// Copyright© 2025 toshio
///
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write, ErrorKind};
use serde::{Serialize, Deserialize};
use candid::{CandidType, Principal};
use sha2::{Sha256, Digest};

const MIMETYPE_DIRECTORY: &str = "canistorage/directory";
const MAX_PATH:usize = 1024;
const MAX_READ_SIZE:usize = 1024 * 1024;

const ERROR_NOT_FOUND: u32 = 1; // File or directory not found
const ERROR_ALREADY_EXISTS: u32 = 2; // Fire or directory already exists
const ERROR_INVALID_PATH: u32 = 3;
const ERROR_INVALID_MIMETYPE: u32 = 4;
const ERROR_PERMISSION_DENIED: u32 = 5;
const ERROR_INVALID_SEQUENCE: u32 = 6;
const ERROR_INVALID_SIZE: u32 = 7;
const ERROR_INVALID_HASH: u32 = 8;
const ERROR_ALREADY_INITIALIZED: u32 = 9;
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
    ic_cdk::api::msg_caller()
}

/////////////////////////////////////////////////////////////////////////////
// Data Structures
/////////////////////////////////////////////////////////////////////////////
#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct Error {
    code:u32,
    message: String,
}
macro_rules! error {
    ($code:expr, $message:expr) => {
        Err(Error {
            code: $code,
            message: $message.to_string(),
        })
    };
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    size: u64,  // bytes
    creator: Principal,
    created_at: u64, // milliseconds
    updater: Principal,
    updated_at: u64, // milliseconds
    mimetype: String,
    manageable: Vec<Principal>, // Grant or Revoke permission
    readable: Vec<Principal>,
    writable: Vec<Principal>,
    sha256: Option<[u8; 32]>,
    signature: Option<Vec<u8>>,
}

impl FileInfo {
    fn is_dir(&self) -> bool {
        self.mimetype == MIMETYPE_DIRECTORY
    }
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct Permission {
    manageable: bool,
    writable: bool,
    readable: bool,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct Info {
    size: u64,  // bytes
    creator: Principal,
    created_at: u64, // milliseconds
    updater: Principal,
    updated_at: u64, // milliseconds
    mimetype: String,
    sha256: Option<[u8; 32]>,
}

struct Uploading {
    owner: Principal,
    size: u64,
    updated_at: u64,
    mimetype: String,
    chunk: HashMap<u64, Vec<u8>>,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct Download {
    size: u64,
    downloaded_at: u64,
    chunk: Vec<u8>,
    sha256: Option<[u8; 32]>, // specified if end of file
}

/////////////////////////////////////////////////////////////////////////////
// Global Variables
/////////////////////////////////////////////////////////////////////////////
thread_local! {
    /// keep uploading temporary data
    static UPLOADING: RefCell<HashMap<String, Uploading>> = RefCell::default();
}


/////////////////////////////////////////////////////////////////////////////
// Methods
/////////////////////////////////////////////////////////////////////////////

/// grants permissions of manage, read, write to tht principal
///
/// # Arguments
///
/// * `path` - must start with ROOT
/// * `principal` - Principal to check
/// * `manageable` - add manage permission if true
/// * `readable` - add readable permission if true
/// * `writable` - add writable permission if true
#[ic_cdk::update(name="addPermission")]
pub fn add_permission(path:String, principal:Principal, manageable:bool, readable:bool, writable:bool) -> Result<(), Error> {
    validate_path(&path)?;

    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_manage_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
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
            set_file_info(&path, &new_info)?;

            Ok(())
        },
        None => error!(ERROR_NOT_FOUND, "File not found")
    }
}

/// revokes permissions of manage, read, write from tht principal
///
/// # Arguments
///
/// * `path` - must start with ROOT
/// * `principal` - Principal to check
/// * `manageable` - revoke manage permission if true
/// * `readable` - revoke read permission if true
/// * `writable` - revoke wrie permission if true
#[ic_cdk::update(name="removePermission")]
pub fn remove_permission(path:String, principal:Principal, manageable:bool, readable:bool, writable:bool) -> Result<(), Error> {
    validate_path(&path)?;

    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_manage_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
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
            set_file_info(&path, &new_info)?;

            Ok(())
        },
        None => error!(ERROR_NOT_FOUND, "File not found") // TODO File or directory
    }
}

/// Returns permissions of the specified path
/// # Arguments
///
/// * `path` - must start with ROOT
///
#[ic_cdk::query(name="hasPermission")]
pub fn has_permission(path:String) -> Result<Permission, Error> {
    validate_path(&path)?;

    let file_info = get_file_info(&path);
    if file_info.is_none() {
        return error!(ERROR_NOT_FOUND, "File not found");
    }

    let caller = caller();

    // TODO optimize algorithm
    Ok(Permission {
        manageable: check_manage_permission(&caller, &path, file_info.as_ref()),
        readable: check_read_permission(&caller, &path, file_info.as_ref()),
        writable: check_write_permission(&caller, &path, file_info.as_ref()),
    })
}

/// Uloads a file to the canister (less than 2MiB)
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
/// * `mimetype` - mimetype of the file
/// * 'data' - file content
/// * 'overwrite' - whether to overwrite the file if it already exists
#[ic_cdk::update]
pub fn save(path:String, mimetype:String, data:Vec<u8>, overwrite:bool) -> Result<(), Error> {
    // First, check path
    validate_path(&path)?;

    // Second, check mimetype
    if mimetype.is_empty() || mimetype == MIMETYPE_DIRECTORY {
        return error!(ERROR_INVALID_MIMETYPE, "Invalid mimetype");
    }

    // Third check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    // Forth Uploading
    let uploading = UPLOADING.with(|uploading| {
        let map = uploading.borrow();
        map.get(&path).is_some() // TODO expired check
    });
    if uploading {
      return error!(ERROR_ALREADY_EXISTS, "File already exists");
    }

    // Fifth, check whether file exists or not
    if file_info.is_some() && overwrite == false {
        return error!(ERROR_ALREADY_EXISTS, "File already exists");
    } else {
        let parent_info = get_file_info(&parent_path(&path));
        if parent_info.is_none() || !parent_info.unwrap().is_dir() {
            return error!(ERROR_NOT_FOUND, "Parent directory not found");
        }
    }

    // save as temp, and then rename it
    let temp_path = temp_path(&path);
    let file = OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path);
    match file {
        Ok(mut file) => {
            match file.write_all(&data) {
                Ok(()) => {
                    let now = time();
                    let info = match file_info {
                        Some(mut info) => {
                            // Update
                            info.size = data.len() as u64;
                            info.updated_at = now;
                            info.mimetype = mimetype;
                            info.sha256 = Some(Sha256::digest(data).into());
                            info.signature = None;
                            info
                        },
                        None => {
                            // New
                            FileInfo {
                                size: data.len() as u64,
                                creator: caller,
                                created_at: now,
                                updater: caller,
                                updated_at: now,
                                mimetype: mimetype,
                                manageable: Vec::new(),
                                readable: Vec::new(),
                                writable: Vec::new(),
                                sha256: Some(Sha256::digest(data).into()),
                                signature: None,
                            }
                        }
                    };

                    match fs::rename(&temp_path, &path) {
                        Ok(_) => {
                            set_file_info(&path, &info)?;
                            Ok(())
                        },
                        Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
                    }
                },
                Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
            }
        },
        Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
    }
}

/// download a file to the canister (less than 2MiB)
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
/// * `start_at` - must start with ROOT and the parent directory must exist

#[ic_cdk::query]
pub fn load(path:String, start_at:u64) -> Result<Download, Error> {
    // First, check path 
    validate_path(&path)?;

    // Second, check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    // Third, check whether file exists or not
    if file_info.is_none() {
        return error!(ERROR_NOT_FOUND, "File not found");
    }

    // FIXME check file size before read to 
    match File::open(path) {
        Ok(mut file) => {
            let mut buffer = [0u8; MAX_READ_SIZE];
            if start_at != 0u64 {
                let _ = file.seek(SeekFrom::Start(start_at)).or_else(|e| error!(ERROR_UNKNOWN, format!("{:?}", e)));
            }
            let readsize = file.read(&mut buffer).or_else(|e| error!(ERROR_UNKNOWN, format!("{:?}", e))).unwrap();
            let downloaded_at = start_at + readsize as u64;
            let info = file_info.unwrap();

            Ok(Download {
                size: info.size,
                downloaded_at,
                chunk: buffer[..readsize].to_vec(),
                sha256: if info.size == downloaded_at {
                    info.sha256
                } else {
                    None
                }
            })
        },
        Err(e) => match e.kind() { // Not expected
            ErrorKind::NotFound => error!(ERROR_NOT_FOUND, "File not found"),
            _ => error!(ERROR_UNKNOWN, format!("{:?}", e))
        }
    }
}

/// starts uploading a file to the canister (more than 2MiB)
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
/// * `mimetype` - mimetype of the file
/// * 'data' - file content
/// * 'overwrite' - whether to overwrite the file if it already exists
#[ic_cdk::update(name="beginUpload")]
pub fn begin_upload(path:String, mimetype:String, overwrite:bool) -> Result<(), Error> {
    // First, check path 
    validate_path(&path)?;

    // Second, check mimetype
    if mimetype.is_empty() || mimetype == MIMETYPE_DIRECTORY {
        return error!(ERROR_INVALID_MIMETYPE, "Invalid mimetype");
    }
    
    // Third check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    // Forth Uploading
    let uploading = UPLOADING.with(|uploading| {
        let map = uploading.borrow();
        map.get(&path).is_some() // TODO expired check
    });
    if uploading {
      return error!(ERROR_ALREADY_EXISTS, "File already exists");
    }

    // Fifth, check whether file exists or not
    if file_info.is_some() && overwrite == false {
        return error!(ERROR_ALREADY_EXISTS, "File already exists");
    } else {
        let parent_info = get_file_info(&parent_path(&path));
        if parent_info.is_none() || !parent_info.unwrap().is_dir() {
            return error!(ERROR_NOT_FOUND, "Parent directory not found");
        }
    }

    UPLOADING.with(|uploading| {
        let mut map = uploading.borrow_mut();

        // Remove expired first
        let now = time();
        map.retain(|_key, value| (value.updated_at + 10 * 60 * 1000) >= now); // expired 10 minutes.

        // Insert entry
        map.insert(path, Uploading{
            owner: caller,
            updated_at: now,
            size: 0,
            mimetype,
            chunk: HashMap::new(),
        });
        Ok(())
    })
}

/// uploads a chunk of the file to the canister
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
/// * `start` - start index
/// * 'data' - chunk of the file
#[ic_cdk::update(name="sendData")]
pub fn send_data(path:String, start:u64, data:Vec<u8>) -> Result<u64, Error> {
    let caller = caller();

    UPLOADING.with(|uploading| {
        let mut map = uploading.borrow_mut();
        match map.get_mut(&path) {
            Some(value) => {
                let now = time();
                if value.owner != caller {
                    error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
                } else if (value.updated_at + 10 * 60 * 1000) < now {
                    error!(ERROR_PERMISSION_DENIED, "session expired")
                } else {
                    value.size += data.len() as u64;
                    value.updated_at = now;

                    // map.try_insert() is still unstable...
                    match value.chunk.insert(start, data) {
                        Some(old) => {
                            // TODO better to be error but currently accepted and overwritten
                            value.size -= old.len() as u64;
                            Ok(value.size)
                        },
                        None => Ok(value.size)
                    }
                }
            },
            None => error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
        }
    })
}

/// commits uploading a file
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
/// * `mimetype` - mimetype of the file
/// * 'data' - file content
/// * 'overwrite' - whether to overwrite the file if it already exists
#[ic_cdk::update(name="commitUpload")]
pub fn commit_upload(path:String, size:u64, sha256:Option<[u8; 32]>) -> Result<(), Error> {
    let caller = caller();

    UPLOADING.with(|uploading| {
        let mut map = uploading.borrow_mut();
        match map.get_mut(&path) {
            Some(value) => {
                let now = time();
                if value.owner != caller {
                    error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
                } else if (value.updated_at + 10 * 60 * 1000) < now {
                    error!(ERROR_PERMISSION_DENIED, "transaction expired")
                } else if value.size != size {
                    error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
                } else {
                    // write file
                    let temp_path = temp_path(&path);
                    let mut hasher = Sha256::new();
                    let mut sha256_verified:Option<[u8; 32]> = None;
                    let result = match fs::File::create(&temp_path) {
                        Ok(file) => {
                            let mut buffer = BufWriter::with_capacity(2*1024*1024, file); // 2MiB Buffer
                            let mut index:u64 = 0;
                            loop {
                                match value.chunk.get(&index) {
                                    Some(data) => {
                                        index += data.len() as u64;
                                        hasher.update(data);
                                        let _result = buffer.write(data); // TODO handling result
                                    },
                                    None => {
                                        if index != size {
                                            return error!(ERROR_INVALID_SIZE, "Invalid size");
                                        }
                                        sha256_verified = Some(hasher.finalize().into());
                                        if sha256.is_some() && sha256_verified.unwrap() != sha256.unwrap() {
                                            return error!(ERROR_INVALID_HASH, "Invalid hash");
                                        }
                                        let _result = buffer.flush(); // TODO handling result
                                        break;
                                    }
                                }
                            }
                            Ok(())
                        },
                        Err(e) => error!(ERROR_UNKNOWN, e) 
                    };
                    match result {
                        Ok(()) => {
                            let file_info = get_file_info(&path);
                            let info = match file_info {
                                Some(mut info) => {
                                    // Update
                                    info.size = size;
                                    info.updated_at = now;
                                    info.mimetype = value.mimetype.clone();
                                    info.sha256 = sha256_verified;
                                    info.signature = None;
                                    info
                                },
                                None => {
                                    // New
                                    FileInfo {
                                        size,
                                        creator: caller,
                                        created_at: now,
                                        updater: caller,
                                        updated_at: now,
                                        mimetype: value.mimetype.clone(),
                                        manageable: Vec::new(),
                                        readable: Vec::new(),
                                        writable: Vec::new(),
                                        sha256: sha256_verified,
                                        signature: None,
                                    }
                                }
                            };

                            match fs::rename(&temp_path, &path) {
                                Ok(_) => {
                                    set_file_info(&path, &info)?;
                                    map.remove(&path);
                                    Ok(())
                                },
                                Err(e) => {
                                    println!("fs::rename failed");
                                    error!(ERROR_UNKNOWN, format!("{:?}", e))
                                }
                            }
                        },
                        Err(e) => Err(e)
                    }
                }
             },
            None => error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
        }
    })
}

/// cancels uploading a file
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
#[ic_cdk::update(name="cancelUpload")]
pub fn cancel_upload(path:String) -> Result<(), Error> {
    let caller = caller();

    UPLOADING.with(|uploading| {
        let mut map = uploading.borrow_mut();
        match map.get(&path) {
            Some(value) => {
                if value.owner != caller {
                    error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
                } else {
                    map.remove(&path);
                    Ok(())
                }
            }
            None => error!(ERROR_INVALID_SEQUENCE, "Invalid sequence")
        }
    })
}

/// deletes a file
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
#[ic_cdk::update(name="delete")]
pub fn delete(path:String) -> Result<(), Error> {
    validate_path(&path)?;

    // Second, check permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    match fs::remove_file(&path) {
        Ok(_) => {
            delete_file_info(&path);

            Ok(())
        },
        Err(e) => match e.kind() {   
            ErrorKind::NotFound => error!(ERROR_NOT_FOUND, "File not found"),
            _=> error!(ERROR_UNKNOWN, format!("{:?}", e))
        }
    }
}

/// returns a list of the files/directories in the specified path
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
#[ic_cdk::query(name="listFiles")]
pub fn list_files(path:String) -> Result<Vec<String>, Error> {
    validate_path(&path)?;

    let file_info = get_file_info(&path);
    let caller = caller();
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    if file_info.is_none() {
        return error!(ERROR_NOT_FOUND, "Directory not found");
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
    Ok(files)
}

/// creates a directory
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
#[ic_cdk::update(name="createDirectory")]
pub fn create_directory(path:String) -> Result<(), Error> {
    validate_path(&path)?;

    // Check write permission
    let caller = caller();
    let file_info = get_file_info(&path);
    if !check_write_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    if file_info.is_some() {
        return error!(ERROR_ALREADY_EXISTS, "Directory already exists"); // FIXME Dir or file exists
    }

    // check parents
    let parent_info = get_file_info(&parent_path(&path));
    if parent_info.is_none() || !parent_info.unwrap().is_dir() {
        return error!(ERROR_NOT_FOUND, "Parent directory not found");
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
                mimetype: MIMETYPE_DIRECTORY.to_string(),
                manageable: Vec::new(),
                readable: Vec::new(),
                writable: Vec::new(),
                sha256: None,
                signature: None,
            })?;

            Ok(())
        },
        Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
    }
}

/// deletes a directory
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
/// * 'recursively' - whether to delete recursively
#[ic_cdk::update(name="deleteDirectory")]
pub fn delete_directory(path:String, recursively:bool) -> Result<(), Error> {
    validate_path(&path)?;

    let file_info = get_file_info(&path);
    let caller = caller();
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    if file_info.is_none() {
        return error!(ERROR_NOT_FOUND, "Directory not found");
    }

    if recursively {
        // delete recursively
        // delete only if empty
        match fs::remove_dir_all(&path) {
            Ok(_) => {
                delete_file_info(&path);
                Ok(())
            },
            Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
        }
    } else {
        // delete only if empty
        match fs::remove_dir(&path) {
            Ok(_) => {
                delete_file_info(&path);
                Ok(())
            },
            Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
        }
    }
}

/// returns a file info
///
/// # Arguments
///
/// * `path` - must start with ROOT and the parent directory must exist
#[ic_cdk::query(name="getInfo")]
pub fn get_info(path:String) -> Result<Info, Error> {
    validate_path(&path)?;

    let file_info = get_file_info(&path);
    let caller = caller();
    if !check_read_permission(&caller, &path, file_info.as_ref()) {
        return error!(ERROR_PERMISSION_DENIED, "Permission denied");
    }

    match file_info {
        Some(info) => Ok(Info {
            size: info.size,
            creator: info.creator,
            created_at: info.created_at,
            updater: info.updater,
            updated_at: info.updated_at,
            mimetype: info.mimetype,
            sha256: info.sha256
        }),
        None => error!(ERROR_NOT_FOUND, "File not found")
    }
}

/// initilizes canistorage
///
/// # Arguments
///
#[ic_cdk::update(name="initCanistorage")]
pub fn init_canistorage() -> Result<(), Error> {
    let root = ROOT.to_string();
    let file_info = get_file_info(&root);
    match file_info {
        Some(_info) => {
            error!(ERROR_ALREADY_INITIALIZED, "Already initialized")
        },
        None => {
            let owner = caller();
            if owner == Principal::anonymous() {
                return error!(ERROR_PERMISSION_DENIED, "Anonymous is not allowed");
            }
            let now = time();
                
            set_file_info(&root, &FileInfo {
                size: 0,
                creator: owner,
                created_at: now,
                updater: owner,
                updated_at: now,
                mimetype: MIMETYPE_DIRECTORY.to_string(),
                manageable: vec![owner],
                readable: vec![owner],
                writable: vec![owner],
                sha256: None,
                signature: None,
            })
        }
    }
}


/////////////////////////////////////////////////////////////////////////////
// Internal functions
/////////////////////////////////////////////////////////////////////////////

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

/// validates the specified path
///
/// # Arguments
///
/// * `path` - path to check
/// 
fn validate_path(path:&String) -> Result<(), Error> {
    // length
    let length = path.len();
    if length == 0 {
        return error!(ERROR_INVALID_PATH, "Path is empty");
    } else if length > MAX_PATH {
        return error!(ERROR_INVALID_PATH, "Path is too long");
    }

    // starts with
    if path.starts_with(ROOT) == false {
        return error!(ERROR_INVALID_PATH, "Not full path");
    }

    // ends with '/' (except root)
    if length > 1 && path.ends_with('/') {
        return error!(ERROR_INVALID_PATH, "Ends with path separator (/)");
    }
    
    // invalid characters
    if ["..", "`"].iter().any(|s| path.contains(s)) {
        return error!(ERROR_INVALID_PATH, "Path contains invalid characters");
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

fn parent_path(path:&String) -> String {
    if path == "/" { // Not expected
        "".to_string()
    } else {
        match path.rfind("/") {
            Some(index) => format!("{}", &path[0..index]),
            None => "".to_string() // not expected
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

fn set_file_info(path:&String, info:&FileInfo) -> Result<(), Error> {
    let info_path = file_info_path(path);
    let file = OpenOptions::new().write(true).create(true).truncate(true).open(&info_path);
    match file {
        Ok(mut file) => {
            match file.write_all(&serde_cbor::to_vec(info).unwrap()) {
                Ok(()) => Ok(()),
                Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
            }
        },
        Err(e) => error!(ERROR_UNKNOWN, format!("{:?}", e))
    }
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


/////////////////////////////////////////////////////////////////////////////
//
// Implementation for PoC only
//
// FIXME Remove before production
#[derive(CandidType, Serialize, Deserialize)]
pub struct FileInfoForPoC {
    size: u64,
    creator: Principal,
    created_at: u64,
    updater: Principal,
    updated_at: u64,
    mimetype: String,
    path: String,
    manageable: Vec<Principal>, // Grant or Revoke permission
    readable: Vec<Principal>,
    writable: Vec<Principal>,
    children: Option<Vec<FileInfoForPoC>>,
}

impl FileInfoForPoC {
    fn is_dir(&self) -> bool {
        self.mimetype == MIMETYPE_DIRECTORY
    }
}

// DEBUG logics for PoC
#[ic_cdk::query(name="getAllInfoForPoC")]
pub fn get_all_info_for_poc() -> Result<FileInfoForPoC, Error> {
    get_info_for_poc(ROOT.to_string())
}

pub fn get_info_for_poc(path:String) -> Result<FileInfoForPoC, Error> {

    match get_file_info(&path) {
        Some(info) => {
            let children = if info.is_dir() {
                // Directory
                let mut children:Vec<FileInfoForPoC> = Vec::new();
                let entries = fs::read_dir(&path).unwrap();
                let _ = entries.map(| entry | {
                    let entry = entry.unwrap();
                    let file_name = entry.path().file_name().unwrap().to_string_lossy().into_owned();
                    if !file_name.starts_with("`") {
                        let file_path = entry.path().to_string_lossy().into_owned();
                        children.push(get_info_for_poc(file_path).unwrap());
                    }
                }).collect::<Vec<()>>();

                children.sort_by(|a, b| 
                    if a.is_dir() {
                        if b.is_dir() {
                            a.path.cmp(&b.path)
                        } else {
                            Ordering::Less
                        }
                    } else if b.is_dir() {
                        Ordering::Greater
                    } else {
                        a.path.cmp(&b.path)
                    }
                );
                Some(children)
            } else {
                // File
                None
            };

            Ok(FileInfoForPoC {
                path,
                size: info.size,
                creator: info.creator,
                created_at: info.created_at,
                updater: info.updater,
                updated_at: info.updated_at,
                mimetype: info.mimetype,
                manageable: info.manageable,
                readable: info.readable,
                writable: info.writable,
                children,
            })
        }
        None => {
            return error!(ERROR_NOT_FOUND, "Directory not found");
        }
    }
}

// DEBUG logics for PoC
#[ic_cdk::update(name="forceResetForPoC")]
pub fn force_reset_for_poc() -> Result<(), Error> {
    // Remove all directories
    let entries = fs::read_dir(&ROOT.to_string()).unwrap();
    let _ = entries.map(| entry | {
        let entry = entry.unwrap();
        let child_path = entry.path().to_string_lossy().into_owned();
        if entry.file_type().unwrap().is_dir() { 
            fs::remove_dir_all(&child_path).unwrap();
        } else {
            fs::remove_file(&child_path).unwrap();
        }
    }).collect::<Vec<()>>();
    Ok(())
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
        // owner
        let owner = Principal::from_text("zebsi-6birt-enaic-v4hbv-zffiv-ft53g-u4gi3-og45y-tskzf-m6jus-xqe").unwrap(); // goddess x 12
        set_caller(owner);

        let _ = fs::remove_dir_all(format!("{}/", ROOT)); // Root is "./.test/" for unit test
        let _ = fs::remove_file(file_info_path(&ROOT.to_string()));
        let _ = fs::create_dir(format!("{}/", ROOT));
        set_file_info(&ROOT.to_string(), &FileInfo {
            size: 0,
            creator: caller(),
            created_at: 0,
            updater: caller(),
            updated_at: 0,
            mimetype: MIMETYPE_DIRECTORY.to_string(),
            manageable: vec![caller()],
            readable: vec![caller()],
            writable: vec![caller()],
            sha256: None,
            signature: None,
        }).unwrap();
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
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), false);
        assert!(result.is_ok());
        let result = load("./.test/file.txt".to_string(), 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().chunk, data);

        // overwrite
        let data = "Hello, World!".as_bytes().to_vec();
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), true);
        assert!(result.is_ok());
        let result = load("./.test/file.txt".to_string(), 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().chunk, data);

        // error
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ERROR_ALREADY_EXISTS);
    }

    #[test]
    fn test_delete() {
        let _context = setup();

        // new file
        let data = "Hello, World!".as_bytes().to_vec();
        let result = save("./.test/file.txt".to_string(), "text/plain".to_string(), data.clone(), false);
        assert!(result.is_ok());
        let result = load("./.test/file.txt".to_string(), 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().chunk, data);

        // delete
        let result = delete("./.test/file.txt".to_string());
        assert!(result.is_ok());

        // delete (File not found)
        let result = delete("./.test/file.txt".to_string());
        assert_eq!(result.unwrap_err().code, ERROR_NOT_FOUND);
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
            mimetype: "".to_string(),
            manageable: Vec::new(),
            readable: vec![principal_readable.clone()],
            writable: vec![principal_writable.clone()],
            sha256: None,
            signature: None,
        };

        // Check of root
        let path = ROOT.to_string();
        set_file_info(&path, &file_info).unwrap();
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
            mimetype: "".to_string(),
            manageable: Vec::new(),
            readable: vec![principal_child_only.clone()],
            writable: vec![principal_child_only.clone()],
            sha256: None,
            signature: None,
        };
        set_file_info(&path, &file_info).unwrap();
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

        // new file
        let data = "Hello, World!".as_bytes().to_vec();
        let result = save("./.test/file".to_string(), "text/plain".to_string(), data.clone(), false);
        assert!(result.is_ok());

        // new folder
        let result = create_directory("./.test/dir".to_string());
        assert!(result.is_ok());

        let result = list_files("./.test".to_string());
        assert!(result.is_ok());
        let list = result.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_add_permission() {
        let _context = setup();
        let owner = caller();

        // user
        let user = Principal::from_text("aaikz-lv7jd-phj2u-t6r4n-6gne4-3rv3x-jus4j-zbiaz-llnsl-jvk5j-iqe").unwrap(); // actor x 12

        // manageable
        set_caller(owner);
        let result = add_permission(ROOT.to_string(), user, true, false, false);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, true);
        assert_eq!(permission.readable, false);
        assert_eq!(permission.writable, false);
        set_caller(owner);
        let result = remove_permission(ROOT.to_string(), user, true, false, false);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, false);
        assert_eq!(permission.readable, false);
        assert_eq!(permission.writable, false);

        // readable
        set_caller(owner);
        let result = add_permission(ROOT.to_string(), user, false, true, false);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, false);
        assert_eq!(permission.readable, true);
        assert_eq!(permission.writable, false);

        set_caller(owner);
        let result = remove_permission(ROOT.to_string(), user, true, true, false);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, false);
        assert_eq!(permission.readable, false);
        assert_eq!(permission.writable, false);

        // writable
        set_caller(owner);
        let result = add_permission(ROOT.to_string(), user, false, false, true);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, false);
        assert_eq!(permission.readable, false);
        assert_eq!(permission.writable, true);

        set_caller(owner);
        let result = remove_permission(ROOT.to_string(), user, true, false, true);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, false);
        assert_eq!(permission.readable, false);
        assert_eq!(permission.writable, false);

        // all
        set_caller(owner);
        let result = add_permission(ROOT.to_string(), user, true, true, true);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, true);
        assert_eq!(permission.readable, true);
        assert_eq!(permission.writable, true);

        // no remove
        set_caller(owner);
        let result = remove_permission(ROOT.to_string(), user, false, false, false);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, true);
        assert_eq!(permission.readable, true);
        assert_eq!(permission.writable, true);

        // remove
        set_caller(owner);
        let result = remove_permission(ROOT.to_string(), user, true, true, true);
        assert!(result.is_ok());
        set_caller(user);
        let permission = has_permission(ROOT.to_string()).unwrap();
        assert_eq!(permission.manageable, false);
        assert_eq!(permission.readable, false);
        assert_eq!(permission.writable, false);
    }

    #[test]
    fn test_remove_permission() {
        // test on test_add_permission()
    }

    #[test]
    fn test_has_permission() {
        // test on test_add_permission()
    }

    #[test]
    fn test_upload() {
        let _context = setup();
        let path = "./.test/file.txt".to_string();
        let result = begin_upload(path.clone(), "text/plain".to_string(), false);
        assert!(result.is_ok());

        let mut index = 0 as u64;
        let data = "AAA".as_bytes().to_vec();
        let result = send_data(path.clone(), index, data.clone());
        index += data.len() as u64;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), index);

        let data = "BBBB".as_bytes().to_vec();
        let result = send_data(path.clone(), index, data.clone());
        index += data.len() as u64;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), index);

        let data = "CCCCC".as_bytes().to_vec();
        let result = send_data(path.clone(), index, data.clone());
        index += data.len() as u64;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), index);

        let expected = "AAABBBBCCCCC".as_bytes();
        assert_eq!(index, expected.len() as u64);
        let result = commit_upload(path.clone(), index, Some(Sha256::digest(expected).into()));
        assert!(result.is_ok());

        let result = load(path.clone(), 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().chunk, expected);
    }

    #[test]
    fn test_load_save_large_file() {
        let _context = setup();

        // save large file
        let path = "./.test/learge_file.bin".to_string();

        // Begin
        let result = begin_upload(path.clone(), "application/octet-stream".to_string(), false);
        assert!(result.is_ok());

        // Send
        let mut index = 0 as u64;
        let mut hasher = Sha256::new();
        for i in "Hello, world".chars() {
            let buffer = vec![i as u8; MAX_READ_SIZE];
            hasher.update(&buffer);
            let result = send_data(path.clone(), index, buffer.to_vec());
            assert!(result.is_ok());
            index += buffer.len() as u64;
            assert_eq!(result.unwrap(), index);
        }

        // Commit
        let result = commit_upload(path.clone(), index, Some(hasher.finalize().into()));
        assert!(result.is_ok());

        // Verify
        let info = get_info(path.clone()).unwrap();
        assert_eq!(info.size, index);

        // Load large file
        let mut start_at = 0;
        let mut hasher = Sha256::new();
        let download = loop {
            let result = load(path.clone(), start_at);
            assert!(result.is_ok());
            let download = result.unwrap();
            start_at = download.downloaded_at;
            hasher.update(&download.chunk);

            if info.size == download.downloaded_at {
                break download;
            }
        };

        assert_eq!(download.sha256.unwrap(), hasher.finalize().as_slice());
    }
}
