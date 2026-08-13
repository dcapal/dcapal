macro_rules! asset_equal_field {
    ($id:expr, $expected:expr, $actual:expr, $field:ident) => {
        if let Some($field) = $expected.$field {
            assert_eq!(
                $field,
                $actual.$field,
                "{} mismatch on `{}`: expected={} actual={}",
                $id,
                stringify!($field),
                $field,
                $actual.$field
            );
        }
    };
}
