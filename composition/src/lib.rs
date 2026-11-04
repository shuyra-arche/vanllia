#![no_std]
//! Product-only composition: one Deployment and the explicit 17 image targets.

pub mod deployments;
pub mod image_builds;
pub mod units;

pub use deployments::{DEPLOYMENTS, PRODUCT};
pub use image_builds::IMAGE_BUILDS;
pub use units::PROGRAMS;
