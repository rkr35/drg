struct FullName<'name> {
    class: &'name str,
    name: &'name str,
    target_outers: List<&'name str, MAX_OUTERS>,
}

impl TryFrom<&str> for FullName {
    fn try_from()
}