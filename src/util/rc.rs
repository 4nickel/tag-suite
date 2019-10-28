use std::rc::Rc;

/// This function probably exists and I was just
/// unable to find it
pub fn identity<T>(rc: Rc<T>) -> T {
    match Rc::try_unwrap(rc) { Ok(v) => v, _ => panic!("Rc error"), }
}
