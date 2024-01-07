use crate::common::base::*;

pub struct Fiber {
    id: NebulaeId,
    owner: Owner,
    asid: NebulaeId,
    stack_base: VirtAddr,
    stack_size: usize,
    security_realm: SecurityRealm,
}