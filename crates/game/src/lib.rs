//! Shared runtime systems for the strategy game and render-gate tools.

// The public crate docs describe internal Bevy systems and trust-tier
// architecture; those links are useful to maintainers even when the linked
// items are not part of the external API.
#![allow(rustdoc::private_intra_doc_links)]

mod app_state;
pub mod input;
pub mod rendering;
pub mod sim;
