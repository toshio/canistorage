/// Canistorage
/// 
/// Copyright© 2025 toshio
///
use std::cell::RefCell;
use candid::Principal;
use ic_stable_structures::{memory_manager::{MemoryId, MemoryManager}, DefaultMemoryImpl};
pub mod canistorage;
use crate::canistorage::{
    Error,
    Permission,
    Info,
    Download,
    FileInfoForPoC, // for PoC
}; // for export_candid!()

/// wasi2ic
const WASI_MEMORY_ID: MemoryId = MemoryId::new(0);

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

#[ic_cdk::init]
fn init() {
    let wasi_memory = MEMORY_MANAGER.with(|m| m.borrow().get(WASI_MEMORY_ID));
    ic_wasi_polyfill::init_with_memory(&[0u8; 32], &[], wasi_memory);
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let wasi_memory = MEMORY_MANAGER.with(|m| m.borrow().get(WASI_MEMORY_ID));
    ic_wasi_polyfill::init_with_memory(&[0u8; 32], &[], wasi_memory);    
}

#[ic_cdk::query]
fn version() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Enable Candid export
ic_cdk_macros::export_candid!();

// Test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), format!("canistorage {}", env!("CARGO_PKG_VERSION")));
    }
}
