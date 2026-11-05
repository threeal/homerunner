//! A toolkit for deploying and managing persistent self-hosted GitHub Actions
//! runners in Docker.
//!
//! The toolkit isn't implemented yet; [`status`] only stands in for it.

/// Returns a notice that the toolkit isn't implemented yet.
#[must_use]
pub const fn status() -> &'static str {
    "homerunner is not implemented yet"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_status() {
        assert_eq!(status(), "homerunner is not implemented yet");
    }
}
