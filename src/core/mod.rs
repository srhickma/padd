//use core::scan::ScanningError;
//use core::parse::ParsingError;
//use std::fmt as stdfmt;

pub mod fmt;
pub mod parse;
pub mod scan;
pub mod spec;
//
//#[derive(Debug)]
//pub enum Error{
//    ScanErr(ScanningError),
//    ParseErr(ParsingError),
//    Err(String),
//}
//
//impl Error{
//    #[allow(dead_code)]
//    fn fmt(&self, f: &mut stdfmt::Formatter) -> stdfmt::Result {
//        write!(f, "{}", self.to_string())
//    }
//    pub fn to_string(&self) -> String {
//        match self {
//            &Error::ScanErr(ref se) => format!("Failed to scan input: No accepting scans after ({},{}): {}...", se.line, se.character, se.sequence),
//            &Error::ParseErr(ref pe) => format!("Failed to parse input: {}", pe.message),
//            &Error::Err(ref msg) => format!("{}", msg),
//        }
//    }
//}