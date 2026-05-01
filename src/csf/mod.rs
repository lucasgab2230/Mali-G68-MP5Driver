//! Command Stream Frontend (CSF) for Mali-G68 MP5
//!
//! The CSF is the hardware command interface for Valhall GPUs. It replaces
//! the older job-based interface (used on Bifrost and older) with a more
//! efficient command stream approach, similar to what desktop GPUs use.
//!
//! ## Architecture
//!
//! The CSF has:
//! - **Groups**: Logical groupings of command queues
//! - **Queues**: Individual command queues for submitting work
//! - **Doorbells**: Mechanism to notify the GPU of new commands
//!
//! ## Queue Layout for Emulators
//!
//! - Queue 0: Graphics (rendering commands)
//! - Queue 1: Compute (async compute for texture decoding, etc.)
//! - Queue 2: Transfer (copy/blit operations)

pub mod queue;
pub mod firmware;

pub use queue::CsfQueue;
pub use firmware::CsfFirmware;