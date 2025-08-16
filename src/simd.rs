use memchr::{memchr, memchr2, memchr3};
use std::arch::x86_64::*;

const XML_TAG_START: u8 = b'<';
const XML_TAG_END: u8 = b'>';
const XML_SLASH: u8 = b'/';
const XML_QUOTE: u8 = b'"';
const XML_EQUALS: u8 = b'=';
const XML_SPACE: u8 = b' ';

#[inline(always)]
pub fn find_tag_start(haystack: &[u8]) -> Option<usize> {
    memchr(XML_TAG_START, haystack)
}

#[inline(always)]
pub fn find_tag_end(haystack: &[u8]) -> Option<usize> {
    memchr(XML_TAG_END, haystack)
}

#[inline(always)]
pub fn find_quote(haystack: &[u8]) -> Option<usize> {
    memchr(XML_QUOTE, haystack)
}

#[inline(always)]
pub fn find_any_delimiter(haystack: &[u8]) -> Option<usize> {
    memchr3(XML_TAG_START, XML_TAG_END, XML_QUOTE, haystack)
}

#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn find_pattern_avx2(haystack: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || haystack.len() < pattern.len() {
        return None;
    }

    let first_byte = _mm256_set1_epi8(pattern[0] as i8);
    let mut i = 0;

    while i + 32 <= haystack.len() {
        let chunk = _mm256_loadu_si256(haystack.as_ptr().add(i) as *const _);
        let cmp = _mm256_cmpeq_epi8(chunk, first_byte);
        let mask = _mm256_movemask_epi8(cmp);

        if mask != 0 {
            for bit_pos in 0..32 {
                if (mask & (1 << bit_pos)) != 0 {
                    let pos = i + bit_pos;
                    if pos + pattern.len() <= haystack.len() 
                        && &haystack[pos..pos + pattern.len()] == pattern {
                        return Some(pos);
                    }
                }
            }
        }
        i += 32;
    }

    while i < haystack.len() - pattern.len() + 1 {
        if &haystack[i..i + pattern.len()] == pattern {
            return Some(i);
        }
        i += 1;
    }

    None
}

#[target_feature(enable = "avx2")]
#[inline]
pub unsafe fn skip_whitespace_avx2(data: &[u8], mut pos: usize) -> usize {
    let space = _mm256_set1_epi8(0x20);
    let tab = _mm256_set1_epi8(0x09);
    let newline = _mm256_set1_epi8(0x0A);
    let carriage = _mm256_set1_epi8(0x0D);

    while pos + 32 <= data.len() {
        let chunk = _mm256_loadu_si256(data.as_ptr().add(pos) as *const _);
        
        let is_space = _mm256_cmpeq_epi8(chunk, space);
        let is_tab = _mm256_cmpeq_epi8(chunk, tab);
        let is_newline = _mm256_cmpeq_epi8(chunk, newline);
        let is_carriage = _mm256_cmpeq_epi8(chunk, carriage);
        
        let is_whitespace = _mm256_or_si256(
            _mm256_or_si256(is_space, is_tab),
            _mm256_or_si256(is_newline, is_carriage)
        );
        
        let mask = _mm256_movemask_epi8(is_whitespace);
        
        if mask != -1 {
            for i in 0..32 {
                if (mask & (1 << i)) == 0 {
                    return pos + i;
                }
            }
        }
        
        pos += 32;
    }

    while pos < data.len() {
        match data[pos] {
            b' ' | b'\t' | b'\n' | b'\r' => pos += 1,
            _ => break,
        }
    }

    pos
}

#[inline(always)]
pub fn skip_whitespace(data: &[u8], mut pos: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && data.len() - pos >= 32 {
            return unsafe { skip_whitespace_avx2(data, pos) };
        }
    }

    while pos < data.len() {
        match data[pos] {
            b' ' | b'\t' | b'\n' | b'\r' => pos += 1,
            _ => break,
        }
    }
    pos
}

#[inline(always)]
pub fn find_pattern(haystack: &[u8], pattern: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && haystack.len() >= 32 {
            return unsafe { find_pattern_avx2(haystack, pattern) };
        }
    }

    haystack.windows(pattern.len())
        .position(|window| window == pattern)
}

pub struct SimdScanner<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> SimdScanner<'a> {
    #[inline(always)]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    #[inline(always)]
    pub fn skip_whitespace(&mut self) {
        self.pos = skip_whitespace(self.data, self.pos);
    }

    #[inline(always)]
    pub fn find_next(&self, byte: u8) -> Option<usize> {
        memchr(byte, &self.data[self.pos..]).map(|i| self.pos + i)
    }

    #[inline(always)]
    pub fn find_pattern(&self, pattern: &[u8]) -> Option<usize> {
        find_pattern(&self.data[self.pos..], pattern).map(|i| self.pos + i)
    }

    #[inline(always)]
    pub fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.data.len());
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    #[inline(always)]
    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    #[inline(always)]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_pattern() {
        let haystack = b"<xbrl:context id=\"c1\">";
        let pattern = b"context";
        assert_eq!(find_pattern(haystack, pattern), Some(6));
    }

    #[test]
    fn test_skip_whitespace() {
        let data = b"   \t\n\r<tag>";
        assert_eq!(skip_whitespace(data, 0), 6);
    }
}
