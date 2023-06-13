//! Functions for packing instruction data into bytes or u32s.

use crate::{
    Imm06,
    Imm12,
    Imm18,
    Imm24,
    RegId,
};

pub(super) fn bytes_from_ra(ra: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra(ra).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb(ra: RegId, rb: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb(ra, rb).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb_rc(ra: RegId, rb: RegId, rc: RegId) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc(ra, rb, rc).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb_rc_rd(
    ra: RegId,
    rb: RegId,
    rc: RegId,
    rd: RegId,
) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc_rd(ra, rb, rc, rd).to_be_bytes())
}

pub(super) fn bytes_from_ra_rb_rc_imm06(
    ra: RegId,
    rb: RegId,
    rc: RegId,
    imm: Imm06,
) -> [u8; 3] {
    u8x3_from_u8x4(u32_from_ra_rb_rc_imm06(ra, rb, rc, imm).to_be_bytes())
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

fn u32_from_imm06(imm: Imm06) -> u32 {
    imm.0 as u32
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

fn u32_from_ra_rb_rc_imm06(ra: RegId, rb: RegId, rc: RegId, imm: Imm06) -> u32 {
    u32_from_ra_rb_rc(ra, rb, rc) | u32_from_imm06(imm)
}

fn u32_from_ra_rb_imm12(ra: RegId, rb: RegId, imm: Imm12) -> u32 {
    u32_from_ra_rb(ra, rb) | u32_from_imm12(imm)
}

fn u32_from_ra_imm18(ra: RegId, imm: Imm18) -> u32 {
    u32_from_ra(ra) | u32_from_imm18(imm)
}

// Ignore the opcode byte, take the remaining instruction data.
fn u8x3_from_u8x4([_, a, b, c]: [u8; 4]) -> [u8; 3] {
    [a, b, c]
}
