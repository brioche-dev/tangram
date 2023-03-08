/// The name of the root module file in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The name of the referenced artifacts directory in an external checkout.
pub const REFERENCED_ARTIFACTS_DIRECTORY_NAME: &str = ".tangram_referenced_artifacts";

/// The number of files than can be open simultaneously.
pub const FILE_SEMAPHORE_SIZE: usize = 16;

/// The number of sockets that can be open simultaneously.
pub const SOCKET_SEMAPHORE_SIZE: usize = 16;
