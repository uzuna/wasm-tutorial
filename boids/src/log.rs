#[macro_export]
macro_rules! info {
    ( $( $t:tt )* ) => {
        web_sys::console::info_1(&format!( $( $t )* ).into());
    }
}

#[macro_export]
macro_rules! error {
    ( $( $t:tt )* ) => {
        web_sys::console::error_1(&format!( $( $t )* ).into());
    }
}
