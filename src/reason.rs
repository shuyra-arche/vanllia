#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Reason {
    Boot,
    Selected,
    RequiredBy(String),
}
