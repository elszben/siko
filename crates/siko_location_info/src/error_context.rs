use crate::file_manager::FileManager;
use crate::item::LocationId;
use crate::location_info::LocationInfo;

pub struct ErrorContext<'a> {
    pub file_manager: &'a FileManager,
    pub location_info: &'a LocationInfo,
}

impl<'a> ErrorContext<'a> {
    pub fn report_error(&self, msg: String, location: LocationId) {
        println!("ERROR: {}", msg); // TODO
    }
}
