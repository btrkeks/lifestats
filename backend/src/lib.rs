// backend/src/lib.rs

pub mod statistics;

pub mod database {
    pub mod db_ops;
    pub mod db_events;
}

pub mod util {
    pub mod paths;
    pub mod string_parsers;
    pub mod to_string;
    pub mod util;
}