use core::cell::Cell;

use crate::structures::bitmap::*;
use crate::common::base::*;
use crate::kernel_statics::UEFI_MEMORY_MAP_1;
use crate::structures::tree::red_black::*;

#[repr(C)]
pub struct MemNode {
    key: Cell<u128>,
    value: Cell<usize>,
    left: Cell<*mut MemNode>,
    right: Cell<*mut MemNode>,
    color: Cell<bool>,
    n: Cell<u128>,
}

impl RBNode for MemNode {
    fn new() -> Self {
        MemNode {
            key: Cell::new(ZERO_U128),
            value: Cell::new(ZERO_USIZE),
            left: Cell::new(core::ptr::null_mut()),
            right: Cell::new(core::ptr::null_mut()),
            color: Cell::new(false),
            n: Cell::new(ZERO_U128),
        }
    }

    fn key(&self) -> u128 {
        self.key.get()
    }

    fn set_key(&self, key: u128) {
        self.key.set(key);
    }

    fn value(&self) -> usize {
        self.value.get()
    }

    fn set_value(&self, value: usize) {
        self.value.set(value);
    }

    fn left(&self) -> *mut Self {
        self.left.get()
    }

    fn set_left(&self, left: *mut Self) {
        self.left.set(left);
    }

    fn right(&self) -> *mut Self {
        self.right.get()
    }

    #[allow(refining_impl_trait)]
    fn set_right(&self, right: *mut Self) {
        self.right.set(right);
    }

    fn color(&self) -> bool {
        self.color.get()
    }

    fn set_color(&self, color: bool) {
        self.color.set(color);
    }

    fn n(&self) -> u128 {
        self.n.get()
    }

    fn set_n(&self, n: u128) {
        self.n.set(n);
    }
}

#[repr(C)]
pub struct MemRegionDescr {
    pub start_addr: Cell<usize>,  // Starting address of the memory frame
    pub size: Cell<usize>,        // Size of the memory frame in bytes
    pub is_free: Cell<bool>,      // Is the frame free?
    pub flags: Cell<usize>,       // Flags for the frame
    pub owner: Cell<Owner>,       // Owner of the frame
    pub idx: Cell<usize>,         // Index of this region in the bitmap
    pub size_node: Cell<MemNode>, // Node for the size tree
    pub addr_node: Cell<MemNode>, // Node for the address tree
}

impl MemRegionDescr {
    pub fn new() -> Self {
        MemRegionDescr {
            start_addr: Cell::new(ZERO_USIZE),
            size: Cell::new(ZERO_USIZE),
            is_free: Cell::new(false),
            flags: Cell::new(ZERO_USIZE),
            owner: Cell::new(Owner::Nobody),
            idx: Cell::new(ZERO_USIZE),
            size_node: Cell::new(MemNode::new()),
            addr_node: Cell::new(MemNode::new()),
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
            size_node: Cell::new(MemNode::new()),
            addr_node: Cell::new(MemNode::new()),
        }
    }

    // returns a pointer to the size node
    pub fn size_node_ptr(&mut self) -> &mut MemNode {
        self.size_node.get_mut()
    }

    // returns a pointer to the address node
    pub fn addr_node_ptr(&mut self) -> &mut MemNode {
        self.addr_node.get_mut()
    }
}

#[allow(dead_code)]
pub struct TreeAllocator {
    // The base address
    phys_base: Cell<PhysAddr>,

    count: Cell<usize>,
    capacity: Cell<usize>,

    rb_size_free: Cell<RBTree<MemNode>>,
    rb_addr_free: Cell<RBTree<MemNode>>,
    rb_size_alloc: Cell<RBTree<MemNode>>,
    rb_addr_alloc: Cell<RBTree<MemNode>>,

    merge_free_dealloc_count: Cell<usize>,
    pub merge_free_dealloc_interval: Cell<usize>,

    bitmap: Bitmap,
}

impl TreeAllocator {
    // Update a region of pages (free/alloc, owner) in our page info structs
    pub fn update_page_info_range(
        &self,
        start_page_base_addr: PhysAddr,
        size: usize,
        is_free: bool,
        owner: Owner,
    ) {
        let start_page_idx = pages::addr_to_page_index(start_page_base_addr);
        let end_page_idx = pages::usize_to_page_index(start_page_base_addr.as_usize() + size - 1);

        for i in start_page_idx..end_page_idx {
            let page_info = unsafe { iron().page_info.as_mut() };
            if page_info.is_none() {
                panic!("Fatal error: page_info is None");
            }

            let pi = page_info.unwrap().as_mut();

            if is_free {
                pi[i].status = pages::PageStatus::Free;
            } else {
                pi[i].status = pages::PageStatus::Alloc;
            }
            pi[i].owner = owner;
        }
    }

    // Add a new region to the tree
    pub fn add_region(
        &mut self,
        start_addr: PhysAddr,
        size: usize,
        is_free: bool,
        flags: usize,
        owner: Owner,
    ) -> Option<usize> {
        let region_slot = self.alloc_new_slot_mut();
        let mem_regions = self.mem_region_array();

        match region_slot {
            Some(idx) => {
                mem_regions[idx].start_addr.set(start_addr.as_usize());
                mem_regions[idx].size.set(size);
                mem_regions[idx].is_free.set(is_free);
                mem_regions[idx].flags.set(flags);
                mem_regions[idx].owner.set(owner);
                mem_regions[idx].idx.set(idx);

                {
                    let size_node = mem_regions[idx].size_node_ptr();
                    size_node.set_value(idx);
                    size_node.set_key(make128(size, start_addr.as_usize()));

                    if is_free {
                        self.rb_size_free.get_mut().put(size_node);
                    } else {
                        self.rb_size_alloc.get_mut().put(size_node);
                    }
                }

                {
                    let addr_node = mem_regions[idx].addr_node_ptr();
                    addr_node.set_value(idx);
                    addr_node.set_key(make128(start_addr.as_usize(), size));

                    if is_free {
                        self.rb_addr_free.get_mut().put(addr_node);
                    } else {
                        self.rb_addr_alloc.get_mut().put(addr_node);
                    }
                }

                self.count.set(self.count.get() + 1);

                Some(idx)
            }
            None => None,
        }
    }

    // completely remove a region from the tree
    pub fn remove_region(&mut self, node_idx: usize) {
        let regions = self.mem_region_array();

        if regions[node_idx].is_free.get() {
            self.rb_size_free
                .get_mut()
                .delete(regions[node_idx].size_node.get_mut().key());
            self.rb_addr_free
                .get_mut()
                .delete(regions[node_idx].addr_node.get_mut().key());
        } else {
            self.rb_size_alloc
                .get_mut()
                .delete(regions[node_idx].size_node.get_mut().key());
            self.rb_addr_alloc
                .get_mut()
                .delete(regions[node_idx].addr_node.get_mut().key());
        }

        self.dealloc_slot(node_idx);

        self.count.set(self.count.get() - 1);
    }

    // get access to the memory region array
    fn mem_region_array(&self) -> &'static mut [MemRegionDescr] {
        unsafe {
            core::slice::from_raw_parts_mut(
                raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(self.phys_base.get()),
                self.capacity.get(),
            )
        }
    }

    // Alloc before doing anything else
    fn alloc_new_slot_mut(&self) -> Option<usize> {
        let new_struct_slot = self.bitmap.find_first_set();

        match new_struct_slot {
            Some(slot) => {
                self.bitmap.clear(slot);
                Some(slot)
            }
            None => None,
        }
    }

    fn dealloc_slot(&self, idx: usize) {
        let mut ptr_mem_region = self.phys_base.get().clone();
        ptr_mem_region.inner_inc_by_type::<MemRegionDescr>(idx);

        let ptr = raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(ptr_mem_region);
        unsafe {
            ptr.write_bytes(ZERO_U8, core::mem::size_of::<MemRegionDescr>());
        }
        self.bitmap.set(idx);
    }

    // find the region that wastes the least amount of space for the specified size
    // if there's any more space than MEMORY_MAX_WASTE, split the region
    pub fn alloc_default_pages(&mut self, size: usize, owner: Owner) -> Option<usize> {
        let new_block = self.rb_size_free.get_mut().ceiling_node(make128(size, 0));

        match new_block {
            Some(block) => {
                let block_idx = unsafe { (*block).value() };
                let mem_regions = self.mem_region_array();

                let block_size = mem_regions[block_idx].size.get();
                let block_start_addr = mem_regions[block_idx].start_addr.get();

                // see if the block is large enough to split
                // split will mark regions as allocated, but if we don't
                // split, we need to mark the region as allocated here
                if block_size - size > MEMORY_MAX_WASTE {
                    // split the block
                    let (old_node_idx, _) =
                        self.split_region_alloc_old(block_idx, size, owner).unwrap();

                    return Some(old_node_idx);
                } else {
                    // mark the block as allocated
                    mem_regions[block_idx].is_free.set(false);
                    mem_regions[block_idx].owner.set(owner);

                    // remove this block from the free tree
                    // and move it over to the alloc tree
                    {
                        let size_node = mem_regions[block_idx].size_node_ptr();
                        self.rb_size_free.get_mut().delete(size_node.key());
                        size_node.set_value(block_idx);
                        size_node.set_key(make128(block_size, block_start_addr));
                        self.rb_size_alloc.get_mut().put(size_node);
                    }

                    {
                        let addr_node = mem_regions[block_idx].addr_node_ptr();
                        self.rb_addr_free.get_mut().delete(addr_node.key());
                        addr_node.set_value(block_idx);
                        addr_node.set_key(make128(block_start_addr, block_size));
                        self.rb_addr_alloc.get_mut().put(addr_node);
                    }

                    self.update_page_info_range(
                        PhysAddr(block_start_addr),
                        block_size,
                        false,
                        owner,
                    );

                    return Some(block_idx);
                }
            }
            None => return None,
        }
    }

    // split a region into two regions
    // returns the old region and the new region
    // the old region is the region that was split into new_size,
    // and the new region is the region that was created from the split
    pub fn split_region_alloc_old(
        &mut self,
        region_idx: usize,
        new_size: usize,
        owner: Owner,
    ) -> Option<(usize, usize)> {
        let mem_regions = self.mem_region_array();

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        // see if the block is large enough to split
        if region_size - new_size > MEMORY_MAX_WASTE {
            // split the block
            let new_region = self.alloc_new_slot_mut();
            match new_region {
                Some(new_region_idx) => {
                    // update the old region
                    mem_regions[region_idx].size.set(new_size);
                    mem_regions[region_idx].is_free.set(false);
                    mem_regions[region_idx].owner.set(owner);
                    mem_regions[region_idx].idx.set(region_idx);

                    // remove the old region from each tree, update the keys, and add it back
                    {
                        let size_node = mem_regions[region_idx].size_node_ptr();
                        let trunk = self.rb_size_free.get_mut();

                        trunk.delete(size_node.key());
                        size_node.set_value(region_idx);
                        size_node.set_key(make128(new_size, region_start_addr));
                        trunk.put(size_node);
                    }

                    {
                        let addr_node = mem_regions[region_idx].addr_node_ptr();
                        self.rb_addr_free.get_mut().delete(addr_node.key());
                        addr_node.set_value(region_idx);
                        addr_node.set_key(make128(region_start_addr, new_size));
                        self.rb_addr_alloc.get_mut().put(addr_node);
                    }

                    self.update_page_info_range(
                        PhysAddr(region_start_addr),
                        new_size,
                        false,
                        owner,
                    );

                    // update the new region
                    mem_regions[new_region_idx]
                        .start_addr
                        .set(region_start_addr + new_size);
                    mem_regions[new_region_idx].size.set(region_size - new_size);
                    mem_regions[new_region_idx].is_free.set(true);
                    mem_regions[new_region_idx].owner.set(Owner::Nobody);
                    mem_regions[new_region_idx].idx.set(new_region_idx);

                    {
                        let size_node = mem_regions[new_region_idx].size_node_ptr();
                        size_node.value.set(new_region_idx);
                        size_node.key.set(make128(
                            region_size - new_size,
                            region_start_addr + new_size,
                        ));
                        self.rb_size_free.get_mut().put(size_node);
                    }

                    {
                        let addr_node = mem_regions[new_region_idx].addr_node_ptr();
                        addr_node.value.set(new_region_idx);
                        addr_node.key.set(make128(
                            region_start_addr + new_size,
                            region_size - new_size,
                        ));
                        self.rb_addr_free.get_mut().put(addr_node);
                    }

                    self.update_page_info_range(
                        PhysAddr(region_start_addr + new_size),
                        region_size - new_size,
                        true,
                        Owner::Nobody,
                    );

                    self.count.set(self.count.get() + 1);

                    Some((region_idx, new_region_idx))
                }
                None => None,
            }
        } else {
            None
        }
    }

    pub fn split_region_alloc_new(
        &mut self,
        region_idx: usize,
        new_size: usize,
        owner: Owner,
    ) -> Option<(usize, usize)> {
        let mem_regions = self.mem_region_array();

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        // see if the block is large enough to split
        if region_size - new_size > MEMORY_MAX_WASTE {
            // split the block
            let new_region = self.alloc_new_slot_mut();
            match new_region {
                Some(new_region_idx) => {
                    // update the old region
                    mem_regions[region_idx].size.set(new_size);
                    mem_regions[region_idx].is_free.set(true);
                    mem_regions[region_idx].owner.set(Owner::Nobody);
                    mem_regions[region_idx].idx.set(region_idx);

                    // remove the old region from each tree

                    {
                        let size_node = mem_regions[region_idx].size_node_ptr();
                        self.rb_size_free.get_mut().delete(size_node.key());
                        size_node.set_value(region_idx);
                        size_node.set_key(make128(new_size, region_start_addr));
                        self.rb_size_free.get_mut().put(size_node);
                    }

                    {
                        let addr_node = mem_regions[region_idx].addr_node_ptr();
                        self.rb_addr_free.get_mut().delete(addr_node.key());
                        addr_node.set_value(region_idx);
                        addr_node.set_key(make128(region_start_addr, new_size));
                        self.rb_addr_free.get_mut().put(addr_node);
                    }

                    // update the new region
                    mem_regions[new_region_idx]
                        .start_addr
                        .set(region_start_addr + new_size);
                    mem_regions[new_region_idx].size.set(region_size - new_size);
                    mem_regions[new_region_idx].is_free.set(false);
                    mem_regions[new_region_idx].owner.set(owner);
                    mem_regions[new_region_idx].idx.set(new_region_idx);

                    {
                        let size_node = mem_regions[new_region_idx].size_node_ptr();
                        size_node.value.set(new_region_idx);
                        size_node.key.set(make128(
                            region_size - new_size,
                            region_start_addr + new_size,
                        ));
                        self.rb_size_alloc.get_mut().put(size_node);
                    }

                    {
                        let addr_node = mem_regions[new_region_idx].addr_node_ptr();
                        addr_node.value.set(new_region_idx);
                        addr_node.key.set(make128(
                            region_start_addr + new_size,
                            region_size - new_size,
                        ));
                        self.rb_addr_alloc.get_mut().put(addr_node);
                    }

                    self.update_page_info_range(
                        PhysAddr(region_start_addr + new_size),
                        region_size - new_size,
                        false,
                        owner,
                    );

                    self.count.set(self.count.get() + 1);

                    Some((region_idx, new_region_idx))
                }
                None => None,
            }
        } else {
            None
        }
    }

    // mark a region as allocated
    pub fn mark_allocated(&mut self, region_idx: usize, owner: Owner) {
        let mem_regions = self.mem_region_array();

        // see if the region is already allocated
        // if it is, then we don't need to do anything
        if mem_regions[region_idx].is_free.get() == false {
            return;
        }

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        mem_regions[region_idx].is_free.set(false);
        mem_regions[region_idx].owner.set(owner);

        // always remove the region from the trees before making
        // any changes to the region's keys, otherwise the rb tree
        // will be corrupted

        {
            let size_node = mem_regions[region_idx].size_node_ptr();
            let trunk = self.rb_size_free.get_mut();

            trunk.delete(size_node.key.get());
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr));
            trunk.put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_node_ptr();
            let trunk = self.rb_addr_free.get_mut();

            trunk.delete(addr_node.key.get());
            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr, region_size));
            trunk.put(addr_node);
        }

        self.update_page_info_range(PhysAddr(region_start_addr), region_size, false, owner);
    }

    // mark a region as free
    pub fn mark_free(&mut self, region_idx: usize) {
        let mem_regions = self.mem_region_array();

        // see if the region is already allocated
        // if it's free, then we don't need to do anything
        if mem_regions[region_idx].is_free.get() == true {
            return;
        }

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        mem_regions[region_idx].is_free.set(true);
        mem_regions[region_idx].owner.set(Owner::Nobody);

        // always remove the region from the trees before making
        // any changes to the region's keys, otherwise the rb tree
        // will be corrupted

        {
            let size_node = mem_regions[region_idx].size_node_ptr();
            self.rb_size_alloc.get_mut().delete(size_node.key.get());
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr));
            self.rb_size_free.get_mut().put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_node_ptr();
            self.rb_addr_alloc.get_mut().delete(addr_node.key.get());
            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr, region_size));
            self.rb_addr_free.get_mut().put(addr_node);
        }

        self.update_page_info_range(
            PhysAddr(region_start_addr),
            region_size,
            true,
            Owner::Nobody,
        );
    }

    // I know all allocations are page aligned; this is for > MEMORY_DEFAULT_PAGE_SIZE or PageSize::Small pages
    pub fn alloc_page_aligned(
        &mut self,
        size: usize,
        owner: Owner,
        page_size: PageSize,
    ) -> Option<usize> {
        let mem_regions = self.mem_region_array();

        // see if we get lucky and find one right off the bat
        let size_key = make128(size, 0);
        let mut block_idx: usize;
        let mut comp_node = self.rb_size_free.get_mut().ceiling_node(size_key);

        while comp_node.is_some() {
            let addr = unsafe { lo64(comp_node.unwrap().as_ref()?.key()) as usize };
            let sz = unsafe { hi64(comp_node.unwrap().as_ref()?.key()) as usize };
            block_idx = unsafe { comp_node.unwrap().as_ref()?.value() };

            if addr.is_page_aligned(page_size) && sz >= size {
                // mark the block as allocated
                mem_regions[block_idx].is_free.set(false);
                mem_regions[block_idx].owner.set(owner);

                // remove this block from the free tree
                // and move it over to the alloc tree
                {
                    let size_node = mem_regions[block_idx].size_node_ptr();
                    self.rb_size_free.get_mut().delete(size_node.key());
                    size_node.set_value(block_idx);
                    size_node.set_key(make128(sz, addr));
                    self.rb_size_alloc.get_mut().put(size_node);
                }
                {
                    let addr_node = mem_regions[block_idx].addr_node_ptr();
                    self.rb_addr_free.get_mut().delete(addr_node.key());
                    addr_node.set_value(block_idx);
                    addr_node.set_key(make128(addr, sz));
                    self.rb_addr_alloc.get_mut().put(addr_node);
                }

                self.update_page_info_range(PhysAddr(addr), sz, false, owner);

                return Some(block_idx);
            } else {
                // see what it would take to align this region
                let aligned_addr = align_up(addr, page_size.into_bits());

                if aligned_addr + size <= addr + sz {
                    // this region can be aligned
                    // split the block
                    // remember, split automatically takes care of marking regions free or
                    // allocated and adding them to the appropriate trunk of the tree
                    let nodes_opt =
                        self.split_region_alloc_new(block_idx, aligned_addr - addr, owner);
                    match nodes_opt {
                        Some((_, new_node_idx)) => {
                            // see if we need to split the block again
                            if mem_regions[new_node_idx].size.get() - size > MEMORY_MAX_WASTE {
                                let nodes_opt2 =
                                    self.split_region_alloc_old(new_node_idx, size, owner);

                                match nodes_opt2 {
                                    Some((old_node_idx2, _)) => {
                                        return Some(old_node_idx2);
                                    }
                                    None => {
                                        panic!("Fatal error splitting memory region");
                                    }
                                }
                            } else {
                                return Some(new_node_idx);
                            }
                        }
                        None => {
                            panic!("Fatal error splitting memory region");
                        }
                    }
                }
            }

            comp_node = self
                .rb_size_free
                .get_mut()
                .ceiling_node(unsafe { comp_node.unwrap().as_ref()?.key() });
        }
        None
    }
}

impl FrameAllocator for TreeAllocator {
    fn new() -> Self {
        TreeAllocator {
            phys_base: Cell::new(PhysAddr(0)),
            count: Cell::new(0),
            capacity: Cell::new(0),

            rb_size_free: Cell::new(RBTree::<MemNode>::new()),
            rb_addr_free: Cell::new(RBTree::<MemNode>::new()),
            rb_size_alloc: Cell::new(RBTree::<MemNode>::new()),
            rb_addr_alloc: Cell::new(RBTree::<MemNode>::new()),

            merge_free_dealloc_count: Cell::new(0),
            merge_free_dealloc_interval: Cell::new(FRAME_ALLOCATOR_COALESCE_THRESHOLD_DEALLOC),

            bitmap: Bitmap::new(Owner::Memory),
        }
    }

    fn init(&mut self) {
        use uefi::table::boot::*;

        let gb = iron();

        self.phys_base
            .set(raw::ptr_to_raw::<MemRegionDescr, PhysAddr>(
                gb.mem_regions as *const MemRegionDescr,
            ));
        self.capacity.set(gb.total_pages);

        // now we need to go through the memory map one final time and add all the regions
        // to their respective trees
        for e in unsafe { UEFI_MEMORY_MAP_1.lock().as_mut().unwrap().entries() } {
            if e.ty == MemoryType::CONVENTIONAL {
                self.add_region(
                    PhysAddr(e.phys_start as usize),
                    e.page_count as usize * MEMORY_DEFAULT_PAGE_USIZE,
                    true,
                    0,
                    Owner::Nobody,
                );
            } else {
                self.add_region(
                    PhysAddr(e.phys_start as usize),
                    e.page_count as usize * MEMORY_DEFAULT_PAGE_USIZE,
                    false,
                    0,
                    Owner::System,
                );
            }
        }
    }

    // Allocates a single page of memory of the specified size (at the proper alignment)
    fn alloc_page(&mut self, owner: Owner, page_size: PageSize) -> Option<PhysAddr> {
        let new_alloc = self.alloc_page_aligned(page_size.into_bits(), owner, page_size);
        let mem_regions = self.mem_region_array();

        match new_alloc {
            Some(region) => {
                let frame_base_addr = mem_regions[region].start_addr.get();
                Some(PhysAddr(frame_base_addr))
            }
            None => None,
        }
    }

    // Deallocates a single page of memory of the specified size
    // page_base is the base address of the page to deallocate, not
    // the base address of the region that contains the page
    fn dealloc_page(&self, _page_base: PhysAddr, _owner: Owner, _page_size: PageSize) {}

    fn free_page_count(&mut self) -> usize {
        self.free_mem_count() / MEMORY_DEFAULT_PAGE_USIZE
    }

    fn free_mem_count(&mut self) -> usize {
        self.rb_size_free.get_mut().sum() as usize
    }

    fn page_count(&self) -> usize {
        iron().total_pages
    }

    fn mem_count(&self) -> usize {
        iron().phys_mem_max.as_usize()
    }

    fn is_memory_frame_free(&self, page_base: PhysAddr) -> bool {
        let mem_regions = self.mem_region_array();
        let page_index = pages::addr_to_page_index(page_base);

        debug_assert!(page_index < iron().total_pages);

        if mem_regions[page_index].is_free.get() {
            return true;
        }

        false
    }

    fn is_frame_index_free(&self, page_idx: usize) -> bool {
        debug_assert!(page_idx < iron().total_pages);

        let mem_regions = self.mem_region_array();

        if mem_regions[page_idx].is_free.get() {
            return true;
        }

        false
    }
}
