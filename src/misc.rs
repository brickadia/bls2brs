// #[macro_export]
macro_rules! map {
    [$($key:expr => $value:expr),* $(,)?] => {
        vec![
            $(
                ($key, $value),
            )*
        ].into_iter().collect()
    }
}

// #[macro_export]
macro_rules! brick_map_literal {
    [$($ui:expr => $map:expr),* $(,)?] => {
        map![
            $($ui => AsBrickDescVec::as_brick_mapping_vec($map),)*
        ]
    }
}

// #[macro_export]
macro_rules! brick_map_regex {
    [$($source:expr => $func:expr),* $(,)?] => {
        vec![
            $(
                (
                    Regex::new($source).expect("failed to compile regex"),
                    Box::new($func),
                ),
            )*
        ]
    }
}
