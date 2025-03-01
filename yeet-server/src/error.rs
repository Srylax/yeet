use std::fmt::Display;

use axum::http::StatusCode;

impl<T, E: Display> WithStatusCode<T> for Result<T, E> {
    fn with_code(self, status_code: StatusCode) -> Result<T, (StatusCode, String)> {
        self.map_err(|err| (status_code, err.to_string()))
    }
}

pub trait WithStatusCode<T> {
    fn with_code(self, status_code: StatusCode) -> Result<T, (StatusCode, String)>;
}
