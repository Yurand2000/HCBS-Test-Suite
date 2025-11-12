pub mod prelude {
    pub use super::generic::prelude::*;
    pub use super::skeleton::prelude::*;
    pub use super::periodic_thread::prelude as periodic_thread;
}

pub mod generic;
pub mod skeleton;
pub mod periodic_thread;