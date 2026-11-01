//! Shared offline fixtures for native coordinator unit tests.
#[path = "../../tests/common/native_adapter.rs"]
pub(crate) mod fake;
#[path = "../../tests/common/native_delivery.rs"]
pub(crate) mod fixture;

#[path = "../../tests/common/native_process.rs"]
pub(crate) mod process_fixture;
