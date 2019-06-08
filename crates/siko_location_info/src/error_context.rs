use crate::file_manager::FileManager;
use crate::location_info::LocationInfo;
use crate::item::LocationId;

pub struct ErrorContext<'a> {
    pub file_manager: &'a FileManager,
    pub location_info: &'a LocationInfo,
}


impl<'a> ErrorContext<'a> {
    pub fn report_error(&self, msg: String, location: LocationId) {

    }
}
