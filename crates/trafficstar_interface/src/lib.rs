#![feature(ip_as_octets)]
pub mod controller;
pub mod reservation;
pub mod cached_vec;
pub mod iptable_rule;
#[cfg(test)]
mod test;

#[derive(PartialEq, Clone)]
pub enum LinkType{
    TOR,
    MULLVAD,
    PURE
}