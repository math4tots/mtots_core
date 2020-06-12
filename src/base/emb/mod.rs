//! Embedded modules
//! These are scripts that could live separately, but seem important
//! enough that they should be embedded directly into the source

use crate::Globals;

pub(super) fn install_embedded_sources(globals: &mut Globals) {
    add(globals, "__prelude", include_str!("prel.u"));
    add(globals, "os", include_str!("os.u"));
}

fn add(globals: &mut Globals, name: &'static str, data: &'static str) {
    globals.add_embedded_source(name.into(), data);
}
