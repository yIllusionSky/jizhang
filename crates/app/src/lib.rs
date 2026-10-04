//! Android host for the wallet application.
#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod rates;
#[cfg(target_os = "android")]
mod ui;

#[cfg(target_os = "android")]
mod documents;
