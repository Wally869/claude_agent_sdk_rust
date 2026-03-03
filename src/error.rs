//! Error types for Claude Agent SDK.


/// Result type alias for Claude SDK operations.
pub type Result<T> = std::result::Result<T, ClaudeSDKError>;

/// Main error type for the Claude Agent SDK.
#[derive(Debug, thiserror::Error)]
pub enum ClaudeSDKError {
    /// Claude Code CLI binary not found.
    #[error("Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code\n\
             If already installed, ensure it's in PATH or set cli_path in options.")]
    CLINotFound,

    /// Failed to connect to or communicate with the CLI.
    #[error("Failed to connect to Claude Code: {0}")]
    CLIConnection(String),

    /// CLI version is below the minimum required version.
    #[error("Claude Code version {found} is below minimum required version {minimum}.\n\
             Update with: npm update -g @anthropic-ai/claude-code")]
    CLIVersionTooOld { found: String, minimum: String },

    /// CLI process failed with an error.
    #[error("CLI process failed with exit code {exit_code}: {message}{}", .stderr.as_ref().map(|s| format!("\nStderr: {}", s)).unwrap_or_default())]
    Process {
        exit_code: i32,
        message: String,
        stderr: Option<String>,
    },

    /// Failed to parse JSON from CLI output.
    #[error("Failed to parse JSON from CLI: {0}")]
    JSONDecode(#[from] serde_json::Error),

    /// Failed to parse a message into a typed structure.
    #[error("Failed to parse message: {0}")]
    MessageParse(String),

    /// Transport is not ready for communication.
    #[error("Transport not ready for communication. Process may not be started.")]
    TransportNotReady,

    /// Client is not connected.
    #[error("Not connected. Call connect() before attempting operations.")]
    NotConnected,

    /// Client is already connected.
    #[error("Already connected. Disconnect first before reconnecting.")]
    AlreadyConnected,

    /// Control protocol request timed out.
    #[error("Control request timed out after {timeout_secs} seconds: {request_type}")]
    ControlTimeout {
        timeout_secs: u64,
        request_type: String,
    },

    /// Hook callback with the given ID was not found.
    #[error("Hook callback not found: {0}")]
    HookNotFound(String),

    /// Permission callback was not set but is required.
    #[error("Permission callback (can_use_tool) is not set but is required for this operation.")]
    PermissionCallbackNotSet,

    /// MCP server with the given name was not found.
    #[error("MCP server '{0}' not found in configuration.")]
    McpServerNotFound(String),

    /// Invalid configuration provided.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// JSON buffer exceeded maximum size.
    #[error("JSON buffer exceeded maximum size of {max_size} bytes. Message may be truncated or malformed.")]
    BufferOverflow { max_size: usize },

    /// IO error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Authentication error (OAuth/API key issues).
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Network error occurred during API request.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Failed to parse data (non-JSON parsing).
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Unknown message type from CLI (forward-compatible skip).
    #[error("Unknown message type: {0}")]
    UnknownMessageType(String),

    /// Generic error with a custom message.
    #[error("{0}")]
    Other(String),
}

impl ClaudeSDKError {
    /// Create a connection error with a custom message.
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        Self::CLIConnection(msg.into())
    }

    /// Create a process error with exit code and message.
    pub fn process(exit_code: i32, message: String, stderr: Option<String>) -> Self {
        Self::Process {
            exit_code,
            message,
            stderr,
        }
    }

    /// Create a message parse error.
    pub fn message_parse<S: Into<String>>(msg: S) -> Self {
        Self::MessageParse(msg.into())
    }

    /// Create a control timeout error.
    pub fn control_timeout(timeout_secs: u64, request_type: String) -> Self {
        Self::ControlTimeout {
            timeout_secs,
            request_type,
        }
    }

    /// Create an invalid config error.
    pub fn invalid_config<S: Into<String>>(msg: S) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Create a buffer overflow error.
    pub fn buffer_overflow(max_size: usize) -> Self {
        Self::BufferOverflow { max_size }
    }

    /// Create a generic error.
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }
}

impl From<String> for ClaudeSDKError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<&str> for ClaudeSDKError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}
