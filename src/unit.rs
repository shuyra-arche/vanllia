mod declaration;

pub use declaration::Declaration;
pub type Unit = unit_contract::document::Unit<String>;
pub type Loader = unit_contract::document::Loader<String>;
pub type Control = unit_contract::document::Control<String>;
pub use unit_contract::document::Creation;
pub use unit_contract::document::Space;
