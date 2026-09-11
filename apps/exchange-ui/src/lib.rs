#[cfg(all(feature = "web", feature = "desktop"))]
compile_error!("features 'web' and 'desktop' are mutually exclusive");

pub mod state;

#[cfg(any(feature = "web", feature = "desktop"))]
pub mod app;
#[cfg(any(feature = "web", feature = "desktop"))]
pub mod client;
#[cfg(any(feature = "web", feature = "desktop"))]
pub mod components;
#[cfg(any(feature = "web", feature = "desktop"))]
pub mod pages;
#[cfg(any(feature = "web", feature = "desktop"))]
pub mod platform;
#[cfg(any(feature = "web", feature = "desktop"))]
pub mod routes;
