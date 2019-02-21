use crate::location_info::filepath::FilePath;
use crate::location_info::location::Location;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone)]
pub struct LocationSet {
    pub file_path: FilePath,
    pub lines: BTreeMap<usize, Vec<Range>>,
}

impl LocationSet {
    pub fn new(file_path: FilePath) -> LocationSet {
        LocationSet {
            file_path: file_path,
            lines: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, location: Location) {
        let ranges = self
            .lines
            .entry(location.line)
            .or_insert_with(|| Vec::new());

        let mut merged = false;

        for range in ranges.iter_mut() {
            if range.end == location.span.start {
                range.end = location.span.end;
                merged = true;
            }
        }

        if !merged {
            ranges.push(Range {
                start: location.span.start,
                end: location.span.end,
            });
        }
    }
}
