pub mod prelude {
    pub use super::generic::prelude::*;
    pub use super::skeleton::prelude::*;
    pub use super::periodic_thread::prelude as periodic_thread;
    pub use super::rt_app::prelude as rt_app;
}

pub mod generic;
pub mod skeleton;
pub mod periodic_thread;
pub mod rt_app;