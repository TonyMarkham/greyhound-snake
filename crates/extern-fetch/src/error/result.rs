use crate::error::Error;

use std::result::Result as StdResult;

pub(crate) type Result<T> = StdResult<T, Error>;
