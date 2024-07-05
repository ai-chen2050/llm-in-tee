use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    //todo add error type
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;
