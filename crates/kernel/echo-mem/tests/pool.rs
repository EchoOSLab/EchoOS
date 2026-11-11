use echo_mem::{BlockPool, MemError, PoolBlock};

#[test]
fn allocates_until_full() {
    let mut pool = BlockPool::<8, 2>::new();
    let a = pool.try_alloc().unwrap();
    let b = pool.try_alloc().unwrap();
    assert_ne!(a.index, b.index);
    assert_eq!(pool.try_alloc(), Err(MemError::OutOfMemory));
    assert_eq!(pool.used_blocks(), 2);
    assert_eq!(pool.free_blocks(), 0);
    assert_eq!(pool.capacity_bytes(), 16);
    assert_eq!(pool.used_bytes(), 16);
}

#[test]
fn free_and_reuse() {
    let mut pool = BlockPool::<8, 2>::new();
    let a = pool.try_alloc().unwrap();
    let _b = pool.try_alloc().unwrap();
    assert_eq!(pool.free(a), Ok(()));
    assert!(!pool.is_allocated(a));
    assert_eq!(pool.try_alloc().unwrap().index, a.index);
}

#[test]
fn double_free_is_rejected() {
    let mut pool = BlockPool::<8, 2>::new();
    let a = pool.try_alloc().unwrap();
    assert_eq!(pool.free(a), Ok(()));
    assert_eq!(pool.free(a), Err(MemError::DoubleFree));
}

#[test]
fn invalid_block_is_rejected() {
    let mut pool = BlockPool::<8, 2>::new();
    assert_eq!(
        pool.free(PoolBlock { index: 2 }),
        Err(MemError::InvalidBlock)
    );
    assert!(pool.block(PoolBlock { index: 2 }).is_none());
}

#[test]
fn block_data_roundtrip() {
    let mut pool = BlockPool::<8, 2>::new();
    let block = pool.try_alloc().unwrap();
    pool.block_mut(block).unwrap()[0] = 0xAA;
    assert_eq!(pool.block(block).unwrap()[0], 0xAA);
    pool.free(block).unwrap();
    assert!(pool.block(block).is_none());
}
