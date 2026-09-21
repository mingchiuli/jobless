mod catalog;
mod id;

pub use catalog::{
    PlatformCatalog, PlatformConfig, PlatformDescriptor, PlatformError, compatibility_platform,
};
pub use id::PlatformId;
