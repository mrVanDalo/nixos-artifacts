// Shared macros for the artifacts-cli crate

// Export the macro at the crate root so it can be used anywhere as `string_vec![]`
#[macro_export]
macro_rules! string_vec {
    ($($x:expr),* $(,)?) => {
        vec![$($x.to_string()),*]
    };
}

// Allow the file to compile even if not directly referenced besides the macro export
#[allow(dead_code)]
const _MACROS_RS: () = ();
