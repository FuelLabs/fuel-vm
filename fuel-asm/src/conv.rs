//! A set of private conversion functions for packing and unpacking instructions to and from u32s
//! and fixed size byte arrays.

use crate::{Imm12, Imm18, Imm24, RegId};

fn ra_from_u32(u: u32) -> RegId {
    RegId::new((u >> 18) as u8)
}

fn rb_from_u32(u: u32) -> RegId {
    RegId::new((u >> 12) as u8)
}

fn rc_from_u32(u: u32) -> RegId {
    RegId::new((u >> 6) as u8)
}

fn rd_from_u32(u: u32) -> RegId {
    RegId::new(u as u8)
}

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

pub(super) fn rd_from_bytes(bs: [u8; 3]) -> RegId {
    rd_from_u32(u32::from_be_bytes(u8x4_from_u8x3(bs)))
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

pub(super) fn ra_rb_imm12_from_bytes(bs: [u8; 3]) -> (RegId, RegId, Imm12) {
    (ra_from_bytes(bs), rb_from_bytes(bs), imm12_from_bytes(bs))
}

pub(super) fn ra_imm18_from_bytes(bs: [u8; 3]) -> (RegId, Imm18) {
    (ra_from_bytes(bs), imm18_from_bytes(bs))
}

fn u32_from_ra(r: RegId) -> u32 {
    (r.0 as u32) << 18
}

fn u32_from_rb(r: RegId) -> u32 {
    (r.0 as u32) << 12
}

fn u32_from_rc(r: RegId) -> u32 {
    (r.0 as u32) << 6
}

fn u32_from_rd(r: RegId) -> u32 {
    r.0 as u32
}

fn u32_from_imm12(imm: Imm12) -> u32 {
    imm.0 as u32
}

fn u32_from_imm18(imm: Imm18) -> u32 {
    imm.0
}

fn u32_from_imm24(imm: Imm24) -> u32 {
    imm.0
}

fn u32_from_ra_rb(ra: RegId, rb: RegId) -> u32 {
    u32_from_ra(ra) | u32_from_rb(rb)
}

fn u32_from_ra_rb_rc(ra: RegId, rb: RegId, rc: RegId) -> u32 {
    u32_from_ra_rb(ra, rb) | u32_from_rc(rc)
}

fn u32_from_ra_rb_rc_rd(ra: RegId, rb: RegId, rc: RegId, rd: RegId) -> u32 {
    u32_from_ra_rb_rc(ra, rb, rc) | u32_from_rd(rd)
}

fn u32_from_ra_rb_imm12(ra: RegId, rb: RegId, imm: Imm12) -> u32 {
    u32_from_ra_rb(ra, rb) | u32_from_imm12(imm)
}

fn u32_from_ra_imm18(ra: RegId, imm: Imm18) -> u32 {
    u32_from_ra(ra) | u32_from_imm18(imm)
}

pub(super) fn bytes_from_ra(ra: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra(ra).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb(ra: RegId, rb: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb(ra, rb).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb_rc(ra: RegId, rb: RegId, rc: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc(ra, rb, rc).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb_rc_rd(ra: RegId, rb: RegId, rc: RegId, rd: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc_rd(ra, rb, rc, rd).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb_imm12(ra: RegId, rb: RegId, imm: Imm12) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_imm12(ra, rb, imm).to_be_bytes())
}

pub(super) fn bytes_from_ra_imm18(ra: RegId, imm: Imm18) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_imm18(ra, imm).to_be_bytes())
}

pub(super) fn bytes_from_imm24(imm: Imm24) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_imm24(imm).to_be_bytes())
}

// Ignore the opcode byte, take the remaining instruction data.
fn u8x3_from_u8x4([_, a, b, c]: [u8; 4]) -> [u8; 3] {
    [a, b, c]
}

// Produce the big-endian bytes for an instruction's data, with a zeroed opcode byte.
fn u8x4_from_u8x3([a, b, c]: [u8; 3]) -> [u8; 4] {
    [0, a, b, c]
}
