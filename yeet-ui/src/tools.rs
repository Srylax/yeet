use std::{fmt::Display, ops::Not, time::Duration};

use crate::TOASTS;

pub trait NotifyFailure {
    fn toast(self) -> Self;
}

impl<T, E: Display> NotifyFailure for Result<T, E> {
    fn toast(self) -> Self {
        if let Err(ref err) = self {
            TOASTS
                .write()
                .error(err.to_string())
                .duration(Some(Duration::from_secs(5)));
        }
        self
    }
}
