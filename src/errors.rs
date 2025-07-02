//! Error hierarchy with severity classification for systematic error handling.

use thiserror::Error;

/// Error severity levels for recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Fatal,
    Critical,
    Recoverable,
    Minor,
}

/// Error types with context and severity classification
#[derive(Error, Debug)]
pub enum VirtualDesktopError {
    #[error("Hyprland IPC connection failed: {source}")]
    IpcConnection {
        #[from]
        source: std::io::Error,
    },

    #[error("Configuration validation failed: {field} = '{value}' - {reason}")]
    Configuration {
        field: String,
        value: String,
        reason: String,
    },

    #[error("Virtual desktop state parsing failed: {context}")]
    StateParsing { context: String },

    #[error("Widget operation failed: {operation} - {details}")]
    WidgetOperation {
        operation: String,
        details: String,
    },

    #[error("Retry limit exceeded: {attempts} attempts failed - {last_error}")]
    RetryExhausted {
        attempts: u32,
        last_error: String,
    },

    #[error("JSON processing failed: {operation} - {source}")]
    JsonError {
        operation: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl VirtualDesktopError {
    /// Get error severity for recovery strategy selection
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::IpcConnection { .. } => ErrorSeverity::Recoverable,
            Self::Configuration { .. } => ErrorSeverity::Fatal,
            Self::StateParsing { .. } => ErrorSeverity::Recoverable,
            Self::WidgetOperation { .. } => ErrorSeverity::Minor,
            Self::RetryExhausted { .. } => ErrorSeverity::Critical,
            Self::JsonError { .. } => ErrorSeverity::Recoverable,
            Self::Internal { .. } => ErrorSeverity::Critical,
        }
    }

    /// Create configuration validation error
    pub fn invalid_config(field: &str, value: &str, reason: &str) -> Self {
        Self::Configuration {
            field: field.to_string(),
            value: value.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create state parsing error
    pub fn parsing_failed(context: &str) -> Self {
        Self::StateParsing {
            context: context.to_string(),
        }
    }

    /// Create widget operation error
    pub fn widget_failed(operation: &str, details: &str) -> Self {
        Self::WidgetOperation {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }

    /// Create JSON processing error with context
    pub fn from_json_error(operation: &str, source: serde_json::Error) -> Self {
        Self::JsonError {
            operation: operation.to_string(),
            source,
        }
    }
}

/// Result type alias using VirtualDesktopError
pub type Result<T> = std::result::Result<T, VirtualDesktopError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_classification() {
        let ipc_error = VirtualDesktopError::IpcConnection {
            source: std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection failed"),
        };
        assert_eq!(ipc_error.severity(), ErrorSeverity::Recoverable);

        let config_error = VirtualDesktopError::invalid_config(
            "format",
            "invalid",
            "missing placeholders"
        );
        assert_eq!(config_error.severity(), ErrorSeverity::Fatal);

        let parsing_error = VirtualDesktopError::parsing_failed("JSON parse error");
        assert_eq!(parsing_error.severity(), ErrorSeverity::Recoverable);

        let widget_error = VirtualDesktopError::widget_failed("update", "widget destroyed");
        assert_eq!(widget_error.severity(), ErrorSeverity::Minor);

        let retry_error = VirtualDesktopError::RetryExhausted {
            attempts: 5,
            last_error: "connection timeout".to_string(),
        };
        assert_eq!(retry_error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_convenience_constructors() {
        let config_error = VirtualDesktopError::invalid_config(
            "retry_max",
            "0",
            "must be positive"
        );
        
        match config_error {
            VirtualDesktopError::Configuration { field, value, reason } => {
                assert_eq!(field, "retry_max");
                assert_eq!(value, "0");
                assert_eq!(reason, "must be positive");
            }
            _ => panic!("Expected Configuration error"),
        }

        let parsing_error = VirtualDesktopError::parsing_failed("test context");
        match parsing_error {
            VirtualDesktopError::StateParsing { context } => {
                assert_eq!(context, "test context");
            }
            _ => panic!("Expected StateParsing error"),
        }

        let widget_error = VirtualDesktopError::widget_failed("test_op", "test_details");
        match widget_error {
            VirtualDesktopError::WidgetOperation { operation, details } => {
                assert_eq!(operation, "test_op");
                assert_eq!(details, "test_details");
            }
            _ => panic!("Expected WidgetOperation error"),
        }
    }

    #[test]
    fn test_error_display() {
        let config_error = VirtualDesktopError::invalid_config(
            "format",
            "{invalid}",
            "unknown placeholder"
        );
        
        let error_string = format!("{}", config_error);
        assert!(error_string.contains("Configuration validation failed"));
        assert!(error_string.contains("format"));
        assert!(error_string.contains("{invalid}"));
        assert!(error_string.contains("unknown placeholder"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_error = serde_json::from_str::<()>("invalid json").unwrap_err();
        let wrapped_error = VirtualDesktopError::from_json_error(
            "parsing config",
            json_error
        );

        match wrapped_error {
            VirtualDesktopError::JsonError { operation, source: _ } => {
                assert_eq!(operation, "parsing config");
            }
            _ => panic!("Expected JsonError"),
        }
    }
}