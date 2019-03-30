use crate::location_info::filepath::FilePath;
use crate::location_info::location::Location;

#[derive(Debug)]
pub struct LocationInfo {
    pub file_path: FilePath,
    pub location: Location,
}

#[derive(Debug)]
pub enum LexerError {
    UnsupportedCharacter(char, LocationInfo),
    InvalidIdentifier(String, LocationInfo),
    General(String, FilePath, Location),
}
