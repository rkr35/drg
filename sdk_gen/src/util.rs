#[macro_export]
macro_rules! assert {
    ($assertion:expr) => {{
        let assertion: bool = $assertion;
        if !assertion {
            core::hint::unreachable_unchecked();
        }
    }};
}