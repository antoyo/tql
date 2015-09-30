use syntax::codemap::Span;

pub struct Error {
    pub message: String,
    pub position: Span,
}

impl Error {
    pub fn new(message: String, position: Span) -> Error {
        Error {
            message: message,
            position: position,
        }
    }
}

pub type SqlResult<T> = Result<T, Vec<Error>>;

pub fn res<T>(result: T, errors: Vec<Error>) -> SqlResult<T> {
    if errors.len() > 0 {
        Err(errors)
    }
    else {
        Ok(result)
    }
}
