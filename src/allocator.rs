use bumpalo::Bump;
use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::Arc;
use parking_lot::Mutex;
use string_interner::{DefaultBackend, Symbol};
use string_interner::symbol::SymbolU32;

const ARENA_SIZE: usize = 64 * 1024 * 1024; // 64MB arenas
const POOL_SIZE: usize = 1024;

#[repr(align(64))]
pub struct ArenaAllocator {
    current: RefCell<Bump>,
    arenas: RefCell<Vec<Bump>>,
    string_interner: Arc<Mutex<string_interner::StringInterner<DefaultBackend>>>,
}

impl ArenaAllocator {
    pub fn new() -> Self {
        Self {
            current: RefCell::new(Bump::with_capacity(ARENA_SIZE)),
            arenas: RefCell::new(Vec::with_capacity(16)),
            string_interner: Arc::new(Mutex::new(string_interner::StringInterner::new())),
        }
    }

    #[inline(always)]
    pub fn alloc<T>(&self, val: T) -> &T {
        unsafe {
            let ptr = self.current.borrow().alloc(val) as *const T;
            &*ptr
        }
    }

    #[inline(always)]
    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &[T] {
        unsafe {
            let ptr = self.current.borrow().alloc_slice_copy(slice) as *const [T];
            &*ptr
        }
    }

    #[inline(always)]
    pub fn alloc_str(&self, s: &str) -> &str {
        unsafe {
            let ptr = self.current.borrow().alloc_str(s) as *const str;
            &*ptr
        }
    }

    #[inline(always)]
    pub fn intern_string(&self, s: &str) -> u32 {
        let mut interner = self.string_interner.lock();
        interner.get_or_intern(s).to_usize() as u32
    }

    #[inline(always)]
    pub fn get_interned(&self, id: u32) -> Option<String> {
        let interner = self.string_interner.lock();
        let symbol = SymbolU32::try_from_usize(id as usize)?;
        interner.resolve(symbol)
            .map(|s| s.to_string())
    }

    pub fn reset(&self) {
        let mut current = self.current.borrow_mut();
        current.reset();
        
        let mut arenas = self.arenas.borrow_mut();
        for arena in arenas.iter_mut() {
            arena.reset();
        }
    }

    pub fn new_arena(&self) {
        let mut arenas = self.arenas.borrow_mut();
        let old = std::mem::replace(&mut *self.current.borrow_mut(), 
                                    Bump::with_capacity(ARENA_SIZE));
        arenas.push(old);
    }
}

pub struct ObjectPool<T> {
    pool: Vec<Box<T>>,
    factory: fn() -> T,
}

impl<T> ObjectPool<T> {
    pub fn new(capacity: usize, factory: fn() -> T) -> Self {
        let mut pool = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            pool.push(Box::new(factory()));
        }
        Self { pool, factory }
    }

    #[inline(always)]
    pub fn acquire(&mut self) -> Box<T> {
        self.pool.pop().unwrap_or_else(|| Box::new((self.factory)()))
    }

    #[inline(always)]
    pub fn release(&mut self, obj: Box<T>) {
        if self.pool.len() < POOL_SIZE {
            self.pool.push(obj);
        }
    }
}

#[repr(C, align(64))]
pub struct StackBuffer<const N: usize> {
    data: [MaybeUninit<u8>; N],
    len: usize,
}

impl<const N: usize> StackBuffer<N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, byte: u8) -> bool {
        if self.len < N {
            self.data[self.len] = MaybeUninit::new(byte);
            self.len += 1;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.data.as_ptr() as *const u8,
                self.len
            )
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocator() {
        let arena = ArenaAllocator::new();
        let s1 = arena.alloc_str("hello");
        let s2 = arena.alloc_str("world");
        assert_eq!(s1, "hello");
        assert_eq!(s2, "world");
    }

    #[test]
    fn test_string_interning() {
        let arena = ArenaAllocator::new();
        let id1 = arena.intern_string("test");
        let id2 = arena.intern_string("test");
        assert_eq!(id1, id2);
        
        let s = arena.get_interned(id1).unwrap();
        assert_eq!(s, "test");
    }
}