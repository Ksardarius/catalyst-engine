#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ContextId(pub u32);

pub const CTX_GAMEPLAY: ContextId = ContextId(1);
pub const CTX_UI: ContextId = ContextId(2);
pub const CTX_VEHICLE: ContextId = ContextId(3);
pub const CTX_DEBUG: ContextId = ContextId(4);


