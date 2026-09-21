//! Inherited agent topology placeholder (M1-01 owns this module).
//!
//! The recurrent mask, motor assignment, and structural checks live in
//! [`topology`]. Weight initialization, dynamics, and learning arrive in
//! M1-02 and later; this module must not grow neural-state or search code
//! ahead of its milestone.

pub mod topology;
