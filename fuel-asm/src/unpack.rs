//! Functions for unpacking instruction data from bytes or u32s.

use crate::{
    Imm06,
    Imm12,
    Imm18,
    Imm24,
    RegId,
};

pub(super) fn rd_from_bytes(bs: [u8; 3]) -> RegId {
    rd_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn imm06_from_bytes(bs: [u8; 3]) -> Imm06 {
    imm06_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn imm12_from_bytes(bs: [u8; 3]) -> Imm12 {
    imm12_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn imm18_from_bytes(bs: [u8; 3]) -> Imm18 {
    imm18_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn imm24_from_bytes(bs: [u8; 3]) -> Imm24 {
    imm24_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn ra_rb_from_bytes(bs: [u8; 3]) -> (RegId, RegId) {
    (ra_from_bytes(bs), rb_from_bytes(bs))
}

pub(super) fn ra_rb_rc_from_bytes(bs: [u8; 3]) -> (RegId, RegId, RegId) {
    (ra_from_bytes(bs), rb_from_bytes(bs), rc_from_bytes(bs))
}

pub(super) fn ra_rb_rc_rd_from_bytes(bs: [u8; 3]) -> (RegId, RegId, RegId, RegId) {
    (
        ra_from_bytes(bs),
        rb_from_bytes(bs),
        rc_from_bytes(bs),
        rd_from_bytes(bs),
    )
}

pub(super) fn ra_rb_rc_imm06_from_bytes(bs: [u8; 3]) -> (RegId, RegId, RegId, Imm06) {
    (
        ra_from_bytes(bs),
        rb_from_bytes(bs),
        rc_from_bytes(bs),
        imm06_from_bytes(bs),
    )
}

pub(super) fn ra_rb_imm12_from_bytes(bs: [u8; 3]) -> (RegId, RegId, Imm12) {
    (ra_from_bytes(bs), rb_from_bytes(bs), imm12_from_bytes(bs))
}

pub(super) fn ra_imm18_from_bytes(bs: [u8; 3]) -> (RegId, Imm18) {
    (ra_from_bytes(bs), imm18_from_bytes(bs))
}

#[allow(clippy::cast_possible_truncation)]
fn ra_from_u32(u: u32) -> RegId {
    RegId::new((u >> 18) as u8)
}

#[allow(clippy::cast_possible_truncation)]
fn rb_from_u32(u: u32) -> RegId {
    RegId::new((u >> 12) as u8)
}

#[allow(clippy::cast_possible_truncation)]
fn rc_from_u32(u: u32) -> RegId {
    RegId::new((u >> 6) as u8)
}

#[allow(clippy::cast_possible_truncation)]
fn rd_from_u32(u: u32) -> RegId {
    RegId::new(u as u8)
}

#[allow(clippy::cast_possible_truncation)]
fn imm06_from_u32(u: u32) -> Imm06 {
    Imm06::new(u as u8)
}

#[allow(clippy::cast_possible_truncation)]
fn imm12_from_u32(u: u32) -> Imm12 {
    Imm12::new(u as u16)
}

fn imm18_from_u32(u: u32) -> Imm18 {
    Imm18::new(u)
}

fn imm24_from_u32(u: u32) -> Imm24 {
    Imm24::new(u)
}

pub(super) fn ra_from_bytes(bs: [u8; 3]) -> RegId {
    ra_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn rb_from_bytes(bs: [u8; 3]) -> RegId {
    rb_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

pub(super) fn rc_from_bytes(bs: [u8; 3]) -> RegId {
    rc_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
}

// Produce the big-endian bytes for an instruction's data, with a zeroed opcode byte.
fn u8x4_from_u8x3([a, b, c]: [u8; 3]) -> [u8; 4] {
    [0, a, b, c]
}
