//! CutZone's autopilot.
//!
//! Raw talking footage in, a captioned and silence-free vertical video out,
//! driven through the engine's own API so that what it does is what a person
//! could do in the window.
//!
//! The modules that decide things are plain functions with no audio, model
//! or project behind them, so their rules are tested exactly:
//! [`cuts`] plans which stretches stay and where any moment lands after the
//! cut; [`captions`] lays transcribed speech out for a phone held upright.

pub mod analyze;
pub mod captions;
pub mod cuts;
pub mod hear;
pub mod run;
