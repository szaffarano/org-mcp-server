use std::path::PathBuf;
use tokio::process::Command;

/// Gets the path to a compiled binary in the target debug directory.
///
/// This function locates the specified binary within the workspace's target/debug
/// directory and handles OS-specific executable suffixes automatically.
///
/// # Arguments
///
/// * `bin_name` - The name of the binary to locate (without file extension)
///
/// # Returns
///
/// A `PathBuf` pointing to the binary location
///
/// # Panics
///
/// Panics if the binary cannot be found at the expected location
///
/// # Examples
///
/// ```rust
/// let server_path = get_binary_path("org-mcp-server");
/// assert!(server_path.exists());
/// ```
pub fn get_binary_path(bin_name: &str) -> PathBuf {
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("debug");

    let binary = format!("{bin_name}{}", std::env::consts::EXE_SUFFIX);
    let target_dir = target_dir.join(binary);

    assert!(target_dir.exists(), "binary path not found: {target_dir:?}");

    target_dir
}

/// Adds cargo-llvm-cov environment variables to a child process command.
///
/// This function transfers all environment variables containing "LLVM" from the
/// current process to the specified command. This is essential for enabling
/// code coverage collection during integration tests when using cargo-llvm-cov.
///
/// # Arguments
///
/// * `cmd` - Mutable reference to the `Command` that will receive the environment variables
///
/// # Examples
///
/// ```rust
/// let mut command = Command::new("my-binary");
/// with_coverage_env(&mut command);
/// // Command now has LLVM coverage environment variables set
/// ```
pub fn with_coverage_env(cmd: &mut Command) {
    for (key, value) in std::env::vars() {
        if key.contains("LLVM") {
            cmd.env(&key, &value);
        }
    }
}

/// Macro to create an MCP service using the pre-compiled binary with a temporary directory.
///
/// This macro simplifies the creation of MCP services for integration testing by:
/// - Locating the org-mcp-server binary
/// - Configuring it with the provided temporary directory
/// - Setting up coverage environment variables
/// - Creating and returning a connected service instance
///
/// # Arguments
///
/// * `$temp_dir` - Expression that evaluates to a temporary directory reference
///
/// # Returns
///
/// A configured MCP service instance ready for testing
///
/// # Errors
///
/// Returns an error if the service connection fails
///
/// # Examples
///
/// ```rust
/// let temp_dir = TempDir::new()?;
/// let service = create_mcp_service!(&temp_dir);
/// // Service is now ready for MCP protocol testing
/// ```
#[macro_export]
macro_rules! create_mcp_service {
    ($temp_dir:expr) => {{
        use rmcp::{
            ServiceExt,
            transport::{ConfigureCommandExt, TokioChildProcess},
        };
        use tracing::error;

        let mut command = tokio::process::Command::new(
            $crate::tools::test_helpers::get_binary_path("org-mcp-server"),
        )
        .configure(|cmd| {
            cmd.args(["--root", $temp_dir.path().to_str().unwrap()]);
        });

        $crate::tools::test_helpers::with_coverage_env(&mut command);

        ().serve(TokioChildProcess::new(command)?)
            .await
            .map_err(|e| {
                error!("Failed to connect to server: {}", e);
                e
            })?
    }};
}

// Note: The macro is exported via #[macro_export] above
