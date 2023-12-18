use core::cell::Cell;

use uefi::table::boot::*;

use crate::bitmap::*;
use crate::common::base::*;
use crate::common::kernel_statics::*;
use crate::structures::tree::*;

#[repr(C)]
pub struct MemRegionDescr {
    pub start_addr: Cell<usize>,             // Starting address of the memory frame
    pub size: Cell<usize>,                   // Size of the memory frame in bytes
    pub is_free: Cell<bool>,                 // Is the frame free?
    pub flags: Cell<usize>,                  // Flags for the frame
    pub owner: Cell<Owner>,                  // Owner of the frame
    pub idx: Cell<usize>,                    // Index of this region in the bitmap
    pub size_node: Cell<RBNode<MemRegionDescr>>, // Node for the size tree
    pub addr_node: Cell<RBNode<MemRegionDescr>>, // Node for the address tree
}

pub const MEM_REGION_DESCR_PER_SMALL_PAGE: usize = MEMORY_DEFAULT_PAGE_USIZE / core::mem::size_of::<MemRegionDescr>();

impl MemRegionDescr {
    pub fn new() -> Self {
        MemRegionDescr {
            start_addr: Cell::new(ZERO_USIZE),
            size: Cell::new(ZERO_USIZE),
            is_free: Cell::new(false),
            flags: Cell::new(ZERO_USIZE),
            owner: Cell::new(Owner::Nobody),
            idx: Cell::new(ZERO_USIZE),
            size_node: Cell::new(RBNode::<MemRegionDescr>::new()),
            addr_node: Cell::new(RBNode::<MemRegionDescr>::new()),
        }
    }

    pub fn new_with(
        start_address: usize,
        size: usize,
        is_free: bool,
        flags: usize,
        owner: Owner,
    ) -> Self {
        MemRegionDescr {
            start_addr: Cell::new(start_address),
            size: Cell::new(size),
            is_free: Cell::new(is_free),
            flags: Cell::new(flags),
            owner: Cell::new(owner),
            idx: Cell::new(ZERO_USIZE),
            size_node: Cell::new(RBNode::<MemRegionDescr>::new()),
            addr_node: Cell::new(RBNode::<MemRegionDescr>::new()),
        }
    }

    pub fn get_size_node_ptr(&self) -> *mut RBNode<MemRegionDescr> {
        self.size_node.get_mut() as *mut RBNode<MemRegionDescr>
    }

    pub fn get_addr_node_ptr(&self) -> *mut RBNode<MemRegionDescr> {
        self.addr_node.get_mut() as *mut RBNode<MemRegionDescr>
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum MemTreeTrunk {
    SizeFree,
    SizeAlloc,
    AddressFree,
    AddressAlloc,
}

#[allow(dead_code)]
pub struct TreeAllocator {
    // The base address 
    phys_base: Cell<PhysAddr>,

    count: Cell<usize>,
    capacity: Cell<usize>,

    rb_size_free: Cell<RBTree<MemRegionDescr>>,
    rb_addr_free: Cell<RBTree<MemRegionDescr>>,
    rb_size_alloc: Cell<RBTree<MemRegionDescr>>,
    rb_addr_alloc: Cell<RBTree<MemRegionDescr>>,

    merge_free_dealloc_count: Cell<usize>,
    pub merge_free_dealloc_interval: Cell<usize>,

    bitmap: *mut Bitmap,
}

impl TreeAllocator {
    
    // Add a new region to the tree
    pub fn add_region(&self, start_addr: PhysAddr, size: usize, is_free: bool, flags: usize, owner: Owner) -> bool {
        let region_slot = self.alloc_new_slot_mut();
        match region_slot {
            Some((region_ptr, idx)) => {
                unsafe {
                    (*region_ptr).start_addr.set(start_addr.as_usize());
                    (*region_ptr).size.set(size);
                    (*region_ptr).is_free.set(is_free);
                    (*region_ptr).flags.set(flags);
                    (*region_ptr).owner.set(owner);
                    (*region_ptr).idx.set(idx);

                    let size_node = (*region_ptr).size_node.get_mut();
                    size_node.value.set(idx);
                    size_node.key.set(make128(size, start_addr.as_usize()));
                    size_node.ptr.set(region_ptr);

                    let addr_node = (*region_ptr).addr_node.get_mut();
                    addr_node.value.set(idx);
                    addr_node.key.set(start_addr.as_usize() as u128);
                    addr_node.ptr.set(region_ptr);

                    if is_free {
                        self.rb_size_free.get_mut().put(size_node);
                        self.rb_addr_free.get_mut().put(addr_node);
                    } else {
                        self.rb_size_alloc.get_mut().put(size_node);
                        self.rb_addr_alloc.get_mut().put(addr_node);
                    }
                }
                
                self.count.set(self.count.get() + 1);

                true
            }
            None => false,
        }
    }

    // completely remove a region from the tree
    pub fn remove_region(&self, node: *mut RBNode<MemRegionDescr>) {
        let region_idx = unsafe { (*node).idx.get() };
        let regions = self.get_mem_region_array();

        unsafe {
            if regions[region_idx].is_free.get() {
                self.rb_size_free.get_mut().delete((*node).key.get());
                self.rb_addr_free.get_mut().delete((*node).key.get());
            } else {
                self.rb_size_alloc.get_mut().delete((*node).key.get());
                self.rb_addr_alloc.get_mut().delete((*node).key.get());
            }
        }

        self.dealloc_slot_by_idx(region_idx);

        self.count.set(self.count.get() - 1);
    }
    
    // get access to the memory region array
    fn get_mem_region_array(&self) -> &'static mut [MemRegionDescr] {
        unsafe { 
            core::slice::from_raw_parts_mut(
                raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(self.phys_base.get()), 
                self.capacity.get()) }
    }

    // Alloc before doing anything else
    fn alloc_new_slot_mut(&self) -> Option<(*mut MemRegionDescr, usize)> {
        let new_struct_slot = unsafe { (*self.bitmap).find_first_set() };
        match new_struct_slot {
            Some(slot) => {
                let mut new_struct_addr = self.phys_base.clone();
                new_struct_addr.get_mut().inner_inc_by_type::<MemRegionDescr>(slot);

                let new_struct_ptr = raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(new_struct_addr.get());
                unsafe { (*self.bitmap).clear(slot); }
                Some((new_struct_ptr, slot))
            }
            None => None,
        }
    }

    // Dealloc after doing everything else
    fn dealloc_slot_by_ptr(&self, ptr: *mut RBNode<MemRegionDescr>) {
        let ptr_bitmap_slot = (raw::ptr_to_usize::<MemRegionDescr>(ptr as *const MemRegionDescr) - self.phys_base.get().as_usize()) / core::mem::size_of::<MemRegionDescr>();
        unsafe { ptr.write_bytes(ZERO_U8, core::mem::size_of::<MemRegionDescr>()); }
        unsafe { (*self.bitmap).set(ptr_bitmap_slot); }
    }

    fn dealloc_slot_by_idx(&self, idx: usize) {
        let mut ptr_mem_region = self.phys_base.get().clone();
        ptr_mem_region.inner_inc_by_type::<MemRegionDescr>(idx);

        let ptr = raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(ptr_mem_region);
        unsafe { ptr.write_bytes(ZERO_U8, core::mem::size_of::<MemRegionDescr>()); }
        unsafe { (*self.bitmap).set(idx); }
    }

    // find the region that wastes the least amount of space for the specified size
    // if there's any more space than MEMORY_MAX_WASTE, split the region
    pub fn alloc(&self, size: usize, owner: Owner) -> Option<*mut MemRegionDescr> {
        let new_block = self.rb_size_free.get_mut().ceiling_node(make128(size, 0));
        match new_block {
            Some(block) => {
                let block_idx = unsafe { (*block).idx.get() };
                let mem_regions = self.get_mem_region_array();

                let block_size = mem_regions[block_idx].size.get();
                let block_start_addr = mem_regions[block_idx].start_addr.get();

                // see if the block is large enough to split
                if block_size - size > MEMORY_MAX_WASTE {
                    // split the block
                    let (old_node, new_node) = 
                        self.split_region(unsafe { (*block).ptr.get() }, size, owner).unwrap();

                    return Some(old_node);
                } else {
                    return unsafe { Some((*block).ptr.get()) };
                }
            },
            None => return None,
        }
    }

    // split a region into two regions
    // returns the old region and the new region
    // the old region is the region that was split into new_size,
    // and the new region is the region that was created from the split
    pub fn split_region(&self, region: *mut MemRegionDescr, new_size: usize, owner: Owner) -> Option<(*mut MemRegionDescr, *mut MemRegionDescr)> {
        let region_idx = unsafe { (*region).idx.get() };
        let mem_regions = self.get_mem_region_array();

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        // see if the block is large enough to split
        if region_size - new_size > MEMORY_MAX_WASTE {
            // split the block
            let new_region = self.alloc_new_slot_mut();
            match new_region {
                Some((new_region_ptr, new_region_idx)) => {
                    // update the old region
                    unsafe {
                        // remove the old region from each tree
                        let size_node = (*region).size_node.get_mut();
                        let addr_node = (*region).addr_node.get_mut();

                        self.rb_size_free.get_mut().delete(size_node.key.get());
                        self.rb_addr_free.get_mut().delete(addr_node.key.get());
                        
                        // update the old region
                        (*region).size.set(new_size);
                        (*region).is_free.set(false);
                        (*region).owner.set(owner);
                        (*region).idx.set(region_idx);
                        
                        size_node.value.set(region_idx);
                        size_node.key.set(make128(new_size, region_start_addr));
                        
                        addr_node.value.set(region_idx);
                        addr_node.key.set(region_start_addr as u128);

                        // add the old region back to the tree with its new info
                        self.rb_size_alloc.get_mut().put(size_node);
                        self.rb_addr_alloc.get_mut().put(addr_node);
                    }

                    // update the new region
                    unsafe {
                        (*new_region_ptr).start_addr.set(region_start_addr + new_size);
                        (*new_region_ptr).size.set(region_size - new_size);
                        (*new_region_ptr).is_free.set(true);
                        (*new_region_ptr).owner.set(Owner::Nobody);
                        (*new_region_ptr).idx.set(new_region_idx);

                        let size_node = (*new_region_ptr).size_node.get_mut();
                        size_node.value.set(new_region_idx);
                        size_node.key.set(make128(region_size - new_size, region_start_addr + new_size));

                        let addr_node = (*new_region_ptr).addr_node.get_mut();
                        addr_node.value.set(new_region_idx);
                        addr_node.key.set((region_start_addr + new_size) as u128);

                        self.rb_size_free.get_mut().put(size_node);
                        self.rb_addr_free.get_mut().put(addr_node);
                    }

                    self.count.set(self.count.get() + 1);

                    Some((region, new_region_ptr))
                }
                None => None,
            }
        } else {
            None
        }
    }

    // mark a region as allocated
    pub fn mark_allocated(&self, region: *mut MemRegionDescr, owner: Owner) {
        let region_idx = unsafe { (*region).idx.get() };
        let mem_regions = self.get_mem_region_array();

        // see if the region is already allocated
        // if it is, then we don't need to do anything
        if mem_regions[region_idx].is_free.get() == false {
            return;
        }

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        unsafe {
            (*region).is_free.set(false);
            (*region).owner.set(owner);

            // always remove the region from the trees before making
            // any changes to the region's keys, otherwise the rb tree
            // will be corrupted
            let size_node = (*region).size_node.get_mut();
            let addr_node = (*region).addr_node.get_mut();

            self.rb_size_free.get_mut().delete(size_node.key.get());
            self.rb_addr_free.get_mut().delete(addr_node.key.get());
            
            // update region keys / values
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr));
            
            addr_node.value.set(region_idx);
            addr_node.key.set(region_start_addr as u128);

            // add the region to the alloc tree with its new info
            self.rb_size_alloc.get_mut().put(size_node);
            self.rb_addr_alloc.get_mut().put(addr_node);
        }
    }

    // mark a region as free
    pub fn mark_free(&self, region: *mut MemRegionDescr) {
        let region_idx = unsafe { (*region).idx.get() };
        let mem_regions = self.get_mem_region_array();

        // see if the region is already allocated
        // if it's free, then we don't need to do anything
        if mem_regions[region_idx].is_free.get() == true {
            return;
        }

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        unsafe {
            (*region).is_free.set(true);
            (*region).owner.set(Owner::Nobody);

            // always remove the region from the trees before making
            // any changes to the region's keys, otherwise the rb tree
            // will be corrupted
            let size_node = (*region).size_node.get_mut();
            let addr_node = (*region).addr_node.get_mut();

            self.rb_size_alloc.get_mut().delete(size_node.key.get());
            self.rb_addr_alloc.get_mut().delete(addr_node.key.get());
            
            // update region keys / values
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr));
            
            addr_node.value.set(region_idx);
            addr_node.key.set(region_start_addr as u128);

            // add the region to the free tree with its new info
            self.rb_size_free.get_mut().put(size_node);
            self.rb_addr_free.get_mut().put(addr_node);
        }
    }

    // find the first region that is aligned to the specified alignment
    pub fn find_page_aligned(&self, size: usize, owner: Owner, page_size: PageSize) -> Option<*mut MemRegionDescr> {
        
    }
}

impl FrameAllocator for TreeAllocator {
    fn new() -> Self {
        TreeAllocator {
            phys_base: Cell::new(PhysAddr(0)),
            count: Cell::new(0),
            capacity: Cell::new(0),

            rb_size_free: Cell::new(RBTree::<MemRegionDescr>::new()),
            rb_addr_free: Cell::new(RBTree::<MemRegionDescr>::new()),
            rb_size_alloc: Cell::new(RBTree::<MemRegionDescr>::new()),
            rb_addr_alloc: Cell::new(RBTree::<MemRegionDescr>::new()),

            merge_free_dealloc_count: Cell::new(0),
            merge_free_dealloc_interval: Cell::new(FRAME_ALLOCATOR_COALESCE_THRESHOLD_DEALLOC),

            bitmap: iron().region_bitmap,
        }
    }

    fn init(&self) {
        let gb = iron();

        self.phys_base.set(raw::ptr_to_raw::<MemRegionDescr, PhysAddr>(gb.mem_regions as *const MemRegionDescr));
        self.capacity.set(gb.total_pages);
    }

    // Allocates a single page of memory of the specified size (at the proper alignment)
    fn alloc_page(&self, owner: Owner, page_size: PageSize) -> Option<PhysAddr> {
        let new_alloc = self.find_page_aligned(page_size.into_bits(), owner, page_size);

        match new_alloc {
            Some(region) => {
                self.mark_allocated(region, owner);
                let frame_base_addr = unsafe { (*region).start_addr.get() };
                Some(PhysAddr(frame_base_addr))
            }
            None => None,
        }
    }

    // Deallocates a single page of memory of the specified size
    // page_base is the base address of the page to deallocate, not
    // the base address of the region that contains the page
    fn dealloc_page(&self, page_base: PhysAddr, owner: Owner, page_size: PageSize) {
        // first we need to find the region that contains this page_base in the alloc trunk
        let root = self.rb_addr_alloc.get_mut().root.get();

        // locate the region that contains this page
        let mut current = root;

        while !current.is_null() {
            if unsafe { (*(*current).ptr.get()).start_addr.get() } <= page_base.as_usize()
                && unsafe { (*(*current).ptr.get()).start_addr.get() + (*(*current).ptr.get()).size.get() } > page_base.as_usize()
            {
                // we found the region that contains this page
                // now we need to mark it as free if the owners match
                // and that the region's size == the page_size specified
                if unsafe { (*(*current).ptr.get()).owner.get() } == owner &&
                   unsafe { (*(*current).ptr.get()).size.get() } == page_size.into_bits() {
                    self.mark_free(current);
                    return;
                } else {
                    return;
                }
            } else {
                current = unsafe { (*current).right_addr.get() };
            }
        }
    }

    fn free_page_count(&self) -> usize {
        self.count_free() / MEMORY_DEFAULT_PAGE_USIZE
    }

    fn free_mem_count(&self) -> usize {
        self.count_free()
    }

    fn page_count(&self) -> usize {
        self.count.get() / MEMORY_DEFAULT_PAGE_USIZE
    }

    fn mem_count(&self) -> usize {
        self.count.get()
    }

    fn alloc_contiguous(&self, size: usize, owner: Owner) -> Option<PhysAddr> {
        let new_alloc = self.find(size, owner);

        match new_alloc {
            Some(region) => {
                self.mark_allocated(region, owner);
                let frame_base_addr = unsafe { (*region).start_addr.get() };
                Some(PhysAddr(frame_base_addr))
            }
            None => None,
        }
    }

    fn dealloc_contiguous(&self, page_base: PhysAddr, size: usize, owner: Owner) {
        // first we need to find the region that contains this page_base in the alloc trunk
        let root = self.rb_addr_alloc.get_mut().root.get();

        // locate the region that contains this page
        let mut current = root;

        while !current.is_null() {
            if unsafe { (*current).start_addr.get() } <= page_base.as_usize()
                && unsafe { (*current).start_addr.get() + (*current).size.get() } > page_base.as_usize()
            {
                // we found the region that contains this page
                // now we need to mark it as free as long as the
                // owners match and the size is the same
                if unsafe { (*current).owner.get() } == owner &&
                   unsafe { (*current).size.get() } == size {
                    self.mark_free(current);
                    return; 
                } else {
                    return;
                }
            }
            else {
                current = unsafe { (*current).right_addr.get() };
            }
        }
    }

    fn is_memory_frame_free(&self, page_base: PhysAddr) -> bool {
        let region = raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(page_base.align_4k());
        unsafe { (*region).is_free.get() }
    }

    fn is_frame_index_free(&self, page_idx: usize) -> bool {
        let mem_addr = PhysAddr(page_idx * MEMORY_DEFAULT_PAGE_USIZE);
        let region = raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(mem_addr);
        unsafe { (*region).is_free.get() }
    }
}