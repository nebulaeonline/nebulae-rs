use core::cell::Cell;

use crate::genesis::*;
use crate::structures::bitmap::*;
use crate::common::base::*;
use crate::structures::tree::red_black::*;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
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

pub type MemIdx = usize;

impl MemNode {
    // this function expects to be called with the root node pointer in the address list
    // i.e. it expects the key in address, size format
    pub fn contains_addr(&self, addr_root: *mut MemNode, addr: impl MemAddr + AsUsize) -> Option<*mut MemNode> {
        let node = self._contains_addr(addr_root, addr);
        if node == core::ptr::null_mut() {
            return None;
        }
        Some(node)
    }

    fn _contains_addr(&self, node: *mut MemNode, addr: impl MemAddr + AsUsize) -> *mut MemNode {
        if node == core::ptr::null_mut() {
            return core::ptr::null_mut();
        }

        let node_key = unsafe { (*node).key() };
        let node_addr = hi64(node_key);
        let node_size = lo64(node_key);

        unsafe {
            if addr.as_usize() >= node_addr.as_usize() && addr.as_usize() <= node_addr.as_usize() + node_size.as_usize() {
                return node;
            }
            if addr.as_usize() > node_addr.as_usize() + node_size.as_usize() {
                return self._contains_addr((*node).right(), addr);
            }
            let left = self._contains_addr((*node).left(), addr);
            if left != core::ptr::null_mut() {
                return left;
            }
            return node;
        }
    }
}

#[repr(C)]
pub struct MemRegionDescr {
    pub start_addr: Cell<PhysAddr>,  // Starting address of the memory frame
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
            start_addr: Cell::new(PhysAddr(0)),
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
        start_address: PhysAddr,
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
    pub fn size_tree_node_ptr(&mut self) -> &mut MemNode {
        self.size_node.get_mut()
    }

    // returns a pointer to the address node
    pub fn addr_tree_node_ptr(&mut self) -> &mut MemNode {
        self.addr_node.get_mut()
    }
}

#[allow(dead_code)]
pub struct TreeAllocator {
    // The base address
    pub phys_base: Cell<PhysAddr>,

    count: Cell<usize>,
    capacity: Cell<usize>,

    rb_size_free: Cell<RBTree<MemNode>>,
    rb_addr_free: Cell<RBTree<MemNode>>,
    rb_size_alloc: Cell<RBTree<MemNode>>,
    rb_addr_alloc: Cell<RBTree<MemNode>>,

    dealloc_since_last_merge_free_count: Cell<usize>,
    pub merge_free_dealloc_interval: Cell<usize>,

    pub bitmap: Option<Bitmap>,
}

impl TreeAllocator {
    // Update a region of pages (free/alloc, owner) in our page info structs
    fn update_page_info_range(
        &self,
        start_page_base_addr: PhysAddr,
        size: usize,
        is_free: bool,
        owner: Owner,
    ) {
        let start_page_idx = pages::addr_to_page_index(start_page_base_addr);
        let end_page_idx = pages::usize_to_page_index(start_page_base_addr.as_usize() + size - 1);

        for i in start_page_idx..end_page_idx {
            let page_info = unsafe { iron().page_info.unwrap().as_mut() };

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

    // Merges the free regions in the tree
    // returns the number of merges that were performed
    fn coalesce_free_space(&mut self) -> usize {
        let mut merge_count: usize = 0;

        // get a reference to the memory region array
        let mem_regions = self.get_mem_region_array_ref_mut();

        // get root node from the free address tree
        let free_addr_root = self.rb_addr_free.get_mut().root();
        
        // if there's no root, then there's no memory to coalesce
        if free_addr_root == core::ptr::null_mut() {
            return 0;
        }

        // set the current node = to the min node
        let mut free_addr_cur = self.rb_addr_free.get_mut().min_node(free_addr_root);

        // if there is no min node, but there was a root node, we have some sort of corruption
        if free_addr_cur == core::ptr::null_mut() {
            panic!("critical error in memory subsystem: failed to obtain reference to minimum node from free address root");
        }

        // iterate & merge
        loop {
            // get the current node's key
            let node_key = unsafe { (*free_addr_cur).key() };

            // get the current node's address
            let node_addr = hi64(node_key);

            // get the current node's size
            let node_size = lo64(node_key);

            // get the current node's index
            let node_mem_idx = unsafe { (*free_addr_cur).value() };

            // get the next node's key
            let next_node_key_result = unsafe { (*free_addr_cur).right().as_ref() };
            if next_node_key_result.is_none() {
                // there is no next node, so we're done
                break;
            }
            let next_node_key = next_node_key_result.unwrap().key();

            // get the next node's address
            let next_node_addr = hi64(next_node_key);

            // get the next node's index
            let next_node_mem_idx = unsafe { (*free_addr_cur).right().as_ref()
                .expect("critical error in memory subsystem: failed to obtain reference to free address root").value() };

            // see if the current node and the next node are adjacent
            if node_addr.as_usize() + node_size.as_usize() == next_node_addr.as_usize() {
                // they are adjacent, so merge them
                let merged_region_result = self.merge_free_regions(node_mem_idx, next_node_mem_idx);

                // make sure the merge was successful
                if merged_region_result.is_none() {
                    panic!("critical error in memory subsystem: failed to merge free regions: {} & {}", node_mem_idx, next_node_mem_idx);
                }

                // update the root node
                free_addr_cur = (*mem_regions[merged_region_result.unwrap()].addr_tree_node_ptr()).left();

                // increment our merge count
                merge_count += 1;

            } else {

                // they are not adjacent, so move to the next node
                free_addr_cur = unsafe { (*free_addr_cur).right() };
            }

            // see if we've reached the end of the tree
            if free_addr_cur == core::ptr::null_mut() {
                break;
            }
        }
        
        // reset the deallocs since the last merge count
        self.dealloc_since_last_merge_free_count.set(0);

        merge_count
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
        let region_slot = self.alloc_internal_slot();
        let mem_regions = self.get_mem_region_array_ref_mut();

        match region_slot {
            Some(idx) => {
                mem_regions[idx].start_addr.set(start_addr);
                mem_regions[idx].size.set(size);
                mem_regions[idx].is_free.set(is_free);
                mem_regions[idx].flags.set(flags);
                mem_regions[idx].owner.set(owner);
                mem_regions[idx].idx.set(idx);

                {
                    let size_node = mem_regions[idx].size_tree_node_ptr();
                    size_node.set_value(idx);
                    size_node.set_key(make128(size, start_addr.as_usize()));

                    if is_free {
                        self.rb_size_free.get_mut().put(size_node);
                    } else {
                        self.rb_size_alloc.get_mut().put(size_node);
                    }
                }

                {
                    let addr_node = mem_regions[idx].addr_tree_node_ptr();
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
    pub fn remove_region(&mut self, region_idx: usize) {
        let mem_regions = self.get_mem_region_array_ref_mut();

        if mem_regions[region_idx].is_free.get() {
            self.rb_size_free
                .get_mut()
                .delete(mem_regions[region_idx].size_node.get_mut().key());
            self.rb_addr_free
                .get_mut()
                .delete(mem_regions[region_idx].addr_node.get_mut().key());
        } else {
            self.rb_size_alloc
                .get_mut()
                .delete(mem_regions[region_idx].size_node.get_mut().key());
            self.rb_addr_alloc
                .get_mut()
                .delete(mem_regions[region_idx].addr_node.get_mut().key());
        }

        self.dealloc_internal_slot(region_idx);

        self.count.set(self.count.get() - 1);
    }

    // get access to the memory region array
    fn get_mem_region_array_ref_mut(&self) -> &'static mut [MemRegionDescr] {
        unsafe {
            core::slice::from_raw_parts_mut(
                raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(self.phys_base.get()),
                self.capacity.get(),
            )
        }
    }

    // Alloc before doing anything else
    fn alloc_internal_slot(&mut self) -> Option<usize> {
        if self.bitmap.is_none() {
            return None;
        }

        let new_struct_slot = self.bitmap.as_mut().unwrap().find_first_set();

        match new_struct_slot {
            Some(slot) => {
                self.bitmap.as_mut().unwrap().clear(slot);
                Some(slot)
            }
            None => None,
        }
    }

    // Dealloc after everything else
    fn dealloc_internal_slot(&mut self, idx: usize) {
        if self.bitmap.is_none() {
            return;
        }

        let mut ptr_mem_region = self.phys_base.get().clone();
        ptr_mem_region.inner_inc_by_type::<MemRegionDescr>(idx);

        let ptr = raw::raw_to_ptr_mut::<MemRegionDescr, PhysAddr>(ptr_mem_region);
        unsafe {
            ptr.write_bytes(ZERO_U8, core::mem::size_of::<MemRegionDescr>());
        }
        self.bitmap.as_mut().unwrap().set(idx);
    }

    // split a region into two regions: a left and a right--
    // as on a number line, the left region is to the left of the right region
    // in terms of addresses (i.e. start_1 < end_1 < start_2 < end_2)
    // this fn returns the left and the right regions or None if the region 
    // could not be split
    fn split_free_region(
        &mut self,
        orig_region_idx: usize,
        new_left_size: usize,
    ) -> Option<(usize, usize)> {
        // we should not be calling this function with a size that is not aligned
        debug_assert!(new_left_size.is_aligned_4k());

        // get our aligned size for the new left region
        let new_left_aligned_size = align_up(new_left_size, MEMORY_DEFAULT_PAGE_USIZE);
        let left_region_idx = orig_region_idx;

        // get a reference to the memory region array
        let mem_region_array = self.get_mem_region_array_ref_mut();

        // get the existing stats
        let region_size = mem_region_array[left_region_idx].size.get();
        let orig_region_start_addr = mem_region_array[left_region_idx].start_addr.get();
        let new_right_region_size = region_size - new_left_aligned_size;

        // debug output
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::split_free_region(): region_idx = {}", left_region_idx);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::split_free_region(): region_size = {}", region_size);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::split_free_region(): region_start_addr = 0x{:0x}", orig_region_start_addr);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::split_free_region(): new_left_aligned_size = {}", new_left_aligned_size);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::split_free_region(): new_right_region_size = {}", new_right_region_size);

        // see if the block is large enough to split
        if region_size - new_left_aligned_size > 0 {
           
            // there will be left over space, so split the block
            
            // obtain an index for the new region into the memory region array;
            // the memory region array is controlled by the bitmap
            let new_right_region = self.alloc_internal_slot();
            
            // make sure we were able to allocate a new region slot
            match new_right_region {
                Some(right_region_idx) => {
                    // remove the old region from each tree, update the key/value, 
                    // and add it back to each tree
                    
                    // the size trunk first
                    {
                        let size_node = mem_region_array[left_region_idx].size_tree_node_ptr();
                        let trunk = self.rb_size_free.get_mut();

                        trunk.delete(size_node.key());
                        size_node.set_value(left_region_idx);
                        size_node.set_key(make128(new_left_aligned_size, orig_region_start_addr.as_usize()));
                        trunk.put(size_node);
                    }

                    // then the address trunk
                    {
                        let addr_node = mem_region_array[left_region_idx].addr_tree_node_ptr();
                        let trunk = self.rb_addr_free.get_mut();

                        trunk.delete(addr_node.key());
                        addr_node.set_value(left_region_idx);
                        addr_node.set_key(make128(orig_region_start_addr.as_usize(), new_left_aligned_size));
                        trunk.put(addr_node);
                    }

                    // update the left region entry
                    mem_region_array[left_region_idx].size.set(new_left_aligned_size);
                    mem_region_array[left_region_idx].is_free.set(true);
                    mem_region_array[left_region_idx].owner.set(Owner::Nobody);
                    mem_region_array[left_region_idx].idx.set(left_region_idx);

                    // update the left region's page entries
                    self.update_page_info_range(
                        orig_region_start_addr,
                        new_left_aligned_size,
                        false,
                        Owner::Nobody,
                    );

                    // update the newly created right region's entry
                    mem_region_array[right_region_idx]
                        .start_addr
                        .set(PhysAddr(orig_region_start_addr.as_usize() + new_left_aligned_size));

                    mem_region_array[right_region_idx].size.set(region_size - new_left_aligned_size);
                    mem_region_array[right_region_idx].is_free.set(true);
                    mem_region_array[right_region_idx].owner.set(Owner::Nobody);
                    mem_region_array[right_region_idx].idx.set(right_region_idx);

                    // update the right region's page entries
                    self.update_page_info_range(
                        PhysAddr(orig_region_start_addr.as_usize() + new_left_aligned_size.as_usize()),
                        region_size - new_left_aligned_size,
                        true,
                        Owner::Nobody,
                    );

                    // add the new region to each tree

                    // first to the size trunk
                    {
                        let size_node = mem_region_array[right_region_idx].size_tree_node_ptr();
                        size_node.value.set(right_region_idx);
                        size_node.key.set(make128(
                            region_size - new_left_aligned_size,
                            orig_region_start_addr.as_usize() + new_left_aligned_size,
                        ));
                        self.rb_size_free.get_mut().put(size_node);
                    }

                    // then to the address trunk
                    {
                        let addr_node = mem_region_array[right_region_idx].addr_tree_node_ptr();
                        addr_node.value.set(right_region_idx);
                        addr_node.key.set(make128(
                            orig_region_start_addr.as_usize() + new_left_aligned_size,
                            region_size - new_left_aligned_size,
                        ));
                        self.rb_addr_free.get_mut().put(addr_node);
                    }

                    // we created a new region, so increment our count
                    self.count.set(self.count.get() + 1);

                    Some((left_region_idx, right_region_idx))
                }
                None => None,
            }
        } else if region_size == new_left_aligned_size {
            // return none since the region does not need to be split
            None
        }
        else {
            // return none since the region is too small to split
            None
        }
    }

    // remove a region from the free trunks
    fn remove_region_from_free_trunk(&mut self, region_idx: usize) {
        debug_assert!(region_idx < self.capacity.get());

        let mem_regions = self.get_mem_region_array_ref_mut();

        {
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            let size_trunk = self.rb_size_free.get_mut();
            size_trunk.delete(size_node.key());
        }
        
        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();
            let addr_trunk = self.rb_addr_free.get_mut();
            addr_trunk.delete(addr_node.key());
        }        
    }

    // remove a region from the alloc'ed trunks
    #[allow(dead_code)]
    fn remove_region_from_alloc_trunks(&mut self, region_idx: usize) {
        debug_assert!(region_idx < self.capacity.get());

        let mem_regions = self.get_mem_region_array_ref_mut();

        {
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            let size_trunk = self.rb_size_alloc.get_mut();
            size_trunk.delete(size_node.key());
        }
        
        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();
            let addr_trunk = self.rb_addr_alloc.get_mut();
            addr_trunk.delete(addr_node.key());
        }        
    }

    // put a region into the free trunks
    fn put_region_into_free_trunks(&mut self, region_idx: usize) {
        debug_assert!(region_idx < self.capacity.get());

        let mem_regions = self.get_mem_region_array_ref_mut();
        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        {
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr.as_usize()));
            
            let size_trunk = self.rb_size_free.get_mut();
            size_trunk.put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();

            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr.as_usize(), region_size));
            
            let addr_trunk = self.rb_addr_free.get_mut();
            addr_trunk.put(addr_node);
        }
    }

    // put a region into the alloc'ed trunks
    #[allow(dead_code)]
    fn put_region_into_alloc_trunks(&mut self, region_idx: usize) {
        debug_assert!(region_idx < self.capacity.get());

        let mem_regions = self.get_mem_region_array_ref_mut();
        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        
        {
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr.as_usize()));

            let size_trunk = self.rb_size_alloc.get_mut();
            size_trunk.put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();
            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr.as_usize(), region_size));

            let addr_trunk = self.rb_addr_alloc.get_mut();
            addr_trunk.put(addr_node);
        }
    }

    // merge two free regions
    fn merge_free_regions(&mut self, left_region_idx: usize, right_region_idx: usize) -> Option<usize> {
        let mem_regions = self.get_mem_region_array_ref_mut();

        // make sure the regions are free
        if !mem_regions[left_region_idx].is_free.get() || !mem_regions[right_region_idx].is_free.get() {
            return None;
        }

        // make sure the regions are adjacent
        if mem_regions[left_region_idx].start_addr.get().as_usize() + mem_regions[left_region_idx].size.get() != mem_regions[right_region_idx].start_addr.get().as_usize() {
            return None;
        }

        // remove the regions from each trunk
        self.remove_region_from_free_trunk(left_region_idx);
        self.remove_region_from_free_trunk(right_region_idx);

        // update the left region entry (this will be the new merged region)
        let new_region_size = mem_regions[left_region_idx].size.get() + mem_regions[right_region_idx].size.get();

        mem_regions[left_region_idx].size.set(new_region_size);
        mem_regions[left_region_idx].is_free.set(true);
        mem_regions[left_region_idx].owner.set(Owner::Nobody);
        mem_regions[left_region_idx].idx.set(left_region_idx);

        // temp var (hopfully the compiler will optimize this away)
        let region_start_addr = mem_regions[left_region_idx].start_addr.get();

        // update the info on the left region's tree nodes
        {
            let size_node = mem_regions[left_region_idx].size_tree_node_ptr();
            size_node.value.set(left_region_idx);
            size_node.key.set(make128(new_region_size, region_start_addr.as_usize()));
        }

        {
            let addr_node = mem_regions[left_region_idx].addr_tree_node_ptr();
            addr_node.value.set(left_region_idx);
            addr_node.key.set(make128(region_start_addr.as_usize(), new_region_size));
        }        
        
        // update the left region's page entries
        self.update_page_info_range(
            mem_regions[left_region_idx].start_addr.get(),
            mem_regions[left_region_idx].size.get(),
            true,
            Owner::Nobody,
        );

        // update the right region's page entries
        self.update_page_info_range(
            mem_regions[right_region_idx].start_addr.get(),
            mem_regions[right_region_idx].size.get(),
            true,
            Owner::Nobody,
        );

        // add the left region back to the free trunks
        self.put_region_into_free_trunks(left_region_idx);

        // remove the right region entry entirely
        self.remove_region(right_region_idx);

        Some(left_region_idx)
    }    
    
    // mark a region as allocated
    pub fn mark_region_allocated(&mut self, region_idx: usize, owner: Owner) {
        let mem_regions = self.get_mem_region_array_ref_mut();

        // make sure we're not out of bounds
        if region_idx >= self.capacity.get() {
            panic!("region_idx out of bounds");
        }

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
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            let size_free_trunk = self.rb_size_free.get_mut();
            let size_alloc_trunk = self.rb_size_alloc.get_mut();

            size_free_trunk.delete(size_node.key.get());
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr.as_usize()));
            size_alloc_trunk.put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();
            let addr_free_trunk = self.rb_addr_free.get_mut();
            let addr_alloc_trunk = self.rb_addr_alloc.get_mut();

            addr_free_trunk.delete(addr_node.key.get());
            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr.as_usize(), region_size));
            addr_alloc_trunk.put(addr_node);
        }
        {
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            let size_free_trunk = self.rb_size_free.get_mut();
            let size_alloc_trunk = self.rb_size_alloc.get_mut();

            size_free_trunk.delete(size_node.key.get());
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr.as_usize()));
            size_alloc_trunk.put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();
            let addr_free_trunk = self.rb_addr_free.get_mut();
            let addr_alloc_trunk = self.rb_addr_alloc.get_mut();

            addr_free_trunk.delete(addr_node.key.get());
            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr.as_usize(), region_size));
            addr_alloc_trunk.put(addr_node);
        }

        // update the page info structs
        self.update_page_info_range(region_start_addr, region_size, false, owner);
    }

    // mark a region as free
    pub fn mark_region_free(&mut self, region_idx: usize, owner: Owner) -> bool {
        let mem_regions = self.get_mem_region_array_ref_mut();

        // make sure we're not out of bounds
        if region_idx >= self.capacity.get() {
            panic!("region_idx out of bounds");
        }

        // make sure the owners match
        if mem_regions[region_idx].owner.get() != owner {
            return false;
        }

        // see if the region is already allocated
        // if it isn't, then we don't need to do anything
        if mem_regions[region_idx].is_free.get() {
            return true;
        }

        let region_size = mem_regions[region_idx].size.get();
        let region_start_addr = mem_regions[region_idx].start_addr.get();

        mem_regions[region_idx].is_free.set(true);
        mem_regions[region_idx].owner.set(Owner::Nobody);

        // always remove the region from the trees before making
        // any changes to the region's keys, otherwise the rb trees
        // will be corrupted

        {
            let size_node = mem_regions[region_idx].size_tree_node_ptr();
            let size_free_trunk = self.rb_size_free.get_mut();
            let size_alloc_trunk = self.rb_size_alloc.get_mut();

            size_alloc_trunk.delete(size_node.key.get());
            size_node.value.set(region_idx);
            size_node.key.set(make128(region_size, region_start_addr.as_usize()));
            size_free_trunk.put(size_node);
        }

        {
            let addr_node = mem_regions[region_idx].addr_tree_node_ptr();
            let addr_free_trunk = self.rb_addr_free.get_mut();
            let addr_alloc_trunk = self.rb_addr_alloc.get_mut();

            addr_alloc_trunk.delete(addr_node.key.get());
            addr_node.value.set(region_idx);
            addr_node.key.set(make128(region_start_addr.as_usize(), region_size));
            addr_free_trunk.put(addr_node);
        }

        // update the page info structs
        self.update_page_info_range(region_start_addr, region_size, true, Owner::Nobody);
        true
    }
}

impl FrameAllocator for TreeAllocator {
    fn new() -> Self {
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::new()");

        let ret = TreeAllocator {
            phys_base: Cell::new(PhysAddr(0)),
            count: Cell::new(0),
            capacity: Cell::new(0),

            rb_size_free: Cell::new(RBTree::<MemNode>::new()),
            rb_addr_free: Cell::new(RBTree::<MemNode>::new()),
            rb_size_alloc: Cell::new(RBTree::<MemNode>::new()),
            rb_addr_alloc: Cell::new(RBTree::<MemNode>::new()),

            dealloc_since_last_merge_free_count: Cell::new(0),
            merge_free_dealloc_interval: Cell::new(FRAME_ALLOCATOR_COALESCE_THRESHOLD_DEALLOC),

            bitmap: Some(Bitmap::new(Owner::Memory)),            
        };

        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::new(): complete");

        ret
    }

    fn init(&mut self) {

        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::init()");

        let gb = iron();

        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::init(): gb = {:#x}", gb as *const Nebulae as usize);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::init(): gb.mem_regions = {:#x}", gb.mem_regions.unwrap_or_else(|| {
            panic!("mem_regions was null"); }).as_mut_ptr() as usize);

        self.phys_base
            .set(raw::ptr_to_raw::<MemRegionDescr, PhysAddr>(
                gb.mem_regions.unwrap() as *const MemRegionDescr,
            ));
        self.capacity.set(gb.total_pages);

        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::init(): init complete");
    }

    // find the region that wastes the least amount of space for the specified size
    // if there's any more space than MEMORY_MAX_WASTE, split the region
    fn alloc_frame_single(&mut self, owner: Owner, page_size: PageSize) -> Option<PhysAddr> {
        
        let aligned_size = page_size.as_usize();

        let new_block = self.rb_size_free.get_mut().ceiling_node(make128(aligned_size, 0));
                
        // if we are allocating larger than the default page frame size, then we 
        // need to coalesce the free space
        if page_size.as_usize() > MEMORY_DEFAULT_PAGE_USIZE {
            self.coalesce_free_space();
        }

        // allocate the block, splitting as necessary
        let new_block_idx = unsafe { new_block.unwrap().as_ref()?.value() };
       
        // see if we found a block that will work

        // tap into our memory region array
        let mem_regions = self.get_mem_region_array_ref_mut();

        // grab the stats of this memory region
        let block_size = mem_regions[new_block_idx].size.get();

        // see if the block is large enough to split
        if block_size - aligned_size > 0 {
            
            // split the block; the original block will be the left block with the new size
            // the right block will be added to the free trunks
            let (left_node_idx, _right_node_idx) =
                self.split_free_region(new_block_idx, aligned_size)
                    .unwrap_or_else(|| {
                        panic!("memory region split failed. this is a fatal error.");
                    });
            
            // allocate the left block
            self.mark_region_allocated(left_node_idx, owner);
            return Some(mem_regions[left_node_idx].start_addr.get());
        } else {
            // the block is too small to split, but it will work
            // mark the region as allocated
            self.mark_region_allocated(new_block_idx, owner);
            return Some(mem_regions[new_block_idx].start_addr.get());
        }
    }

    // superceded
    // Allocates a single page of memory of the specified size
    // fn alloc_frame(&mut self, owner: Owner, page_size: PageSize) -> Option<PhysAddr> {
    //     #[cfg(debug_assertions)]
    //     serial_println!("entering TreeAllocator::alloc_page()");

    //     let new_alloc = 
    //         self.alloc_page_aligned_frame(
    //             page_size.as_usize(),
    //             page_size, 
    //             owner);

    //     let mem_regions = self.get_mem_region_array_ref_mut();
        
    //     match new_alloc {
    //         Some(region) => {
    //             let frame_base_addr = mem_regions[region].start_addr.get();
    //             #[cfg(debug_assertions)]
    //             serial_println!("exiting TreeAllocator::alloc_page(): 0x{:0x}", mem_regions[region].start_addr.get());
    //             return Some(PhysAddr(frame_base_addr));
    //         },
    //         None => {
    //             #[cfg(debug_assertions)]
    //             serial_println!("exiting TreeAllocator::alloc_page(): NULLPTR");
    //             return None
    //         },
    //     }        
    // }

    // Allocates memory by physical address & size
    fn alloc_frame_fixed(&mut self, phys_addr: PhysAddr, size: usize, owner: Owner, page_size: PageSize) -> Option<PhysAddr> {
        
        debug_assert!(phys_addr.is_aligned(page_size.as_usize()));
        debug_assert!(size.is_aligned(page_size.as_usize()));

        // align the address to the page size
        let aligned_size = size.align_up(page_size.as_usize());

        // quick stats
        let size_in_pages = pages::calc_pages_reqd(aligned_size, page_size);
        let size_in_bytes = pages::pages_to_bytes(size_in_pages, page_size);

        // on a fixed allocation, we need to coalesce the free blocks
        self.coalesce_free_space();
        
        // We need to see if the frame containing the desired address is free
        let addr_free_root_mem_node_result = 
            self.rb_addr_free
                .get_mut()
                .root();

        // if we don't have a root node right now, that means we have no free memory.
        // don't panic, just hope no one notices.
        if addr_free_root_mem_node_result.is_null() {
            return None;
        }

        // get a reference to the root node of the address tree free trunk
        let addr_free_root_mem_node_ref = unsafe { addr_free_root_mem_node_result.as_ref().unwrap() };
        let addr_free_root_mem_node = unsafe { addr_free_root_mem_node_result.as_mut().unwrap() };
        
        // Find the memory node that contains the address
        let mut containing_node_result = addr_free_root_mem_node_ref.contains_addr(
            addr_free_root_mem_node,
            phys_addr,
        );

        // see if we got anything
        if containing_node_result.is_none() {
            return None;
        } else if containing_node_result.as_mut().unwrap().is_null() {
            return None;
        }

        // we just verified the result is non-none, and non-null, so we can unwrap
        let containing_node = unsafe { containing_node_result.unwrap().as_mut().unwrap() };

        // Now we need to know if this frame is large enough for the proposed allocation
        
        // get a link to the memory region array
        let mem_regions = self.get_mem_region_array_ref_mut();
        let containing_node_region_idx = containing_node.value();
        
        // get the stats of the memory region
        let region_start_addr = mem_regions[containing_node_region_idx].start_addr.get();
        let region_size = mem_regions[containing_node_region_idx].size.get();
        
        // calculate the offset of the desired address from the base address of the region
        let phys_addr_offset = phys_addr.as_usize() - region_start_addr.as_usize();

        // see if the region is too small to contain the desired address
        if region_size - phys_addr_offset < size_in_bytes {
            // the region, even after coalescing free space, is not large enough
            // we must fail the allocation
            return None;
        }
        else {
            // if there's no offset we check then split.
            // if there is an offset, we always split, then check for a 2nd round.

            // we know the region is large enough at this point
            if phys_addr_offset == 0 {
                // there is no offset, so we can allocate the region directly
              
                // see if we need to split the region
                if region_size - size_in_bytes > 0 {
                    // there is excess capacity, so split the block
                    // this time, we will allocate the left block
                    let (left_node_idx, _right_node_idx) = 
                        self.split_free_region(
                            containing_node_region_idx, 
                            size_in_bytes)
                            .unwrap_or_else(|| {
                                panic!("memory region split failed. this is a fatal error.");
                            });

                    // mark the region as allocated
                    self.mark_region_allocated(left_node_idx, owner);
                } else {
                    // the region and the allocation request are the same size;
                    // mark the region as allocated
                    self.mark_region_allocated(containing_node_region_idx, owner);
                }
                // mark the region as allocated
                self.mark_region_allocated(containing_node_region_idx, owner);                               
            } else {
                // get a new region slot
                let new_region = self.alloc_internal_slot();

                // make sure we got a new region slot
                if new_region.is_none() {
                    return None;
                }

                // split the offset into a new region (so offset region will be left, allocated region will be right)
                let (_left_node_idx, right_node_idx) = 
                    self.split_free_region(containing_node_region_idx, phys_addr_offset)
                        .unwrap_or_else(|| {
                            panic!("memory region split failed. this is a fatal error.");
                        });

                // get the stats of the new region
                let new_region_size = mem_regions[right_node_idx].size.get();

                // see if the new region is large enough to split
                if new_region_size - size_in_bytes > 0 {
                    // there is excess capacity, so split the block
                    // this time, we will allocate the left block
                    let (left_node_idx_2, _right_node_idx_2) = 
                        self.split_free_region(
                            right_node_idx, 
                            size_in_bytes)
                            .unwrap_or_else(|| {
                                panic!("memory region split failed. this is a fatal error.");
                            });

                    // mark the region as allocated
                    self.mark_region_allocated(left_node_idx_2, owner);
                } else {
                    // mark the region as allocated
                    self.mark_region_allocated(containing_node_region_idx, owner);
                }
            }            
        }
        Some(phys_addr)
    }

    // general purpose frame allocation
    fn alloc_frame(
        &mut self,
        size: usize,
        page_size: PageSize,
        owner: Owner,
    ) -> Option<PhysAddr> {
        
        debug_assert!(size.is_aligned(page_size.as_usize()));

        // if the page_size is greater than the default page size, then we need to coalesce
        if page_size.as_usize() > MEMORY_DEFAULT_PAGE_USIZE {
            self.coalesce_free_space();
        }

        let mem_regions = self.get_mem_region_array_ref_mut();
        let aligned_size = align_up(size, page_size.as_usize());

        // see if we get lucky and find one right off the bat
        let size_key = make128(aligned_size, 0);
        let mut block_idx: usize;
        let mut comp_node = self.rb_size_free.get_mut().ceiling_node(size_key);

        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::alloc_page_aligned(): size = {}", aligned_size);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::alloc_page_aligned(): size_key = 0x{:0x}", size_key);
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::alloc_page_aligned(): comp_node = 0x{:0x}", unsafe { comp_node.unwrap().as_ref()?.key() });

        while comp_node.is_some() {
            let addr = unsafe { lo64(comp_node.unwrap().as_ref()?.key()) as usize };
            let sz = unsafe { hi64(comp_node.unwrap().as_ref()?.key()) as usize };
            block_idx = unsafe { comp_node.unwrap().as_ref()?.value() };

            if addr.is_aligned(page_size.as_usize()) && sz >= aligned_size {
                
                if sz == aligned_size {
                    // if the page size is exact, then we don't need to split

                    // mark the block as allocated
                    self.mark_region_allocated(block_idx, owner);

                } else {

                    // split the block
                    let nodes_opt =
                        self.split_free_region(block_idx, aligned_size);

                    match nodes_opt {
                        Some((left_node_idx, _right_node_idx)) => {
                            self.mark_region_allocated(left_node_idx, owner);
                            return Some(mem_regions[left_node_idx].start_addr.get());
                        }
                        None => {
                            return None;
                        }
                    }
                }              
            } else {
                // see what it would take to align this region
                let aligned_addr = align_up(addr, page_size.as_usize());

                if aligned_addr + aligned_size <= addr + sz {
                    // this region can be aligned
                    
                    // split the block
                    // the first split will take off the lower addresses to bring the base address of
                    // the region up to the alignment boundary
                    let nodes_opt =
                        self.split_free_region(block_idx, aligned_addr - addr);

                    match nodes_opt {

                        Some((_, right_node_idx)) => {
                            
                            // see if we need to split the block again
                            if mem_regions[right_node_idx].size.get() - aligned_size > 0 {
                                
                                let nodes_opt2 =
                                    self.split_free_region(right_node_idx, aligned_size);

                                match nodes_opt2 {
                                    Some((left_node_idx2, _)) => {
                                        self.mark_region_allocated(left_node_idx2, owner);
                                        return Some(mem_regions[left_node_idx2].start_addr.get());
                                    }
                                    None => {
                                        panic!("could not re-split memory region (old alloc)");
                                    }
                                }
                            } else {
                                // the block is too small to split, but it will work
                                // mark the region as allocated
                                self.mark_region_allocated(right_node_idx, owner);
                                return Some(mem_regions[right_node_idx].start_addr.get());
                            }
                        }
                        None => {
                            panic!("could not split memory region (new alloc)");
                        }
                    }
                }
            }

            comp_node = self
                .rb_size_free
                .get_mut()
                .ceiling_node(unsafe { comp_node.unwrap().as_ref()?.key() + 1 });
        }
        None
    }

    // Deallocates a single page of memory of the specified size
    // page_base is the base address of the page to deallocate, not
    // the base address of the region that contains the page
    fn dealloc_frame(&mut self, page_base: PhysAddr, owner: Owner) {
        #[cfg(debug_assertions)]
        serial_println!("TreeAllocator::dealloc_page(): page_base = 0x{:0x}", page_base.as_usize());

        // get a reference to the memory region array
        let mem_regions = self.get_mem_region_array_ref_mut();

        // get the index of the region that contains the page
        //let region_idx = pages::addr_to_page_index(page_base);

        // get a reference to the root node of the address tree alloc trunk
        let addr_alloc_root_mem_node_result = 
            self.rb_addr_alloc
                .get_mut()
                .root();

        // if there is no root node in the alloc address tree, we haven't
        // allocated any memory, so we can just return
        if addr_alloc_root_mem_node_result.is_null() {
            panic!("TreeAllocator::dealloc_page(): no memory to deallocate");
        }

        // unwrap our ref
        let addr_alloc_root_mem_node_ref = unsafe { addr_alloc_root_mem_node_result.as_ref().unwrap() };
        let addr_alloc_root_mem_node = unsafe { addr_alloc_root_mem_node_result.as_mut().unwrap() };

        // Find the memory node that contains the address
        let mut containing_node_result = addr_alloc_root_mem_node_ref.contains_addr(
            addr_alloc_root_mem_node,
            page_base,
        );

        // see if we got anything
        if containing_node_result.is_none() {
            panic!("TreeAllocator::dealloc_page(): no memory to deallocate");
        } else if containing_node_result.as_mut().unwrap().is_null() {
            panic!("TreeAllocator::dealloc_page(): no memory to deallocate");
        }

        // we just verified the result is non-none, and non-null, so we can unwrap
        let containing_node = unsafe { containing_node_result.unwrap().as_mut().unwrap() };
        let region_to_dealloc_idx = containing_node.value();

        // make sure the owners match
        if mem_regions[region_to_dealloc_idx].owner.get() != owner {
            panic!("dealloc_frame(): owner mismatch -> {:?} tried to deallocate physical memory owned by {:?}", owner, mem_regions[region_to_dealloc_idx].owner.get());
        }

        // get the stats of the region
        let region_size = mem_regions[region_to_dealloc_idx].size.get();
        let region_start_addr = mem_regions[region_to_dealloc_idx].start_addr.get();

        // mark the region as free
        self.mark_region_free(region_to_dealloc_idx, owner);

        // see if we need to coalesce the free space
        if self.dealloc_since_last_merge_free_count.get() >= self.merge_free_dealloc_interval.get() {
            self.coalesce_free_space();
            self.dealloc_since_last_merge_free_count.set(0);
        } else {
            self.dealloc_since_last_merge_free_count.set(self.dealloc_since_last_merge_free_count.get() + 1);
        }

        // update the page info structs
        self.update_page_info_range(region_start_addr, region_size, true, Owner::Nobody);
    }

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
        let mem_regions = self.get_mem_region_array_ref_mut();
        let page_index = pages::addr_to_page_index(page_base);

        debug_assert!(page_index < iron().total_pages);

        if mem_regions[page_index].is_free.get() {
            return true;
        }

        false
    }

    fn is_frame_index_free(&self, page_idx: usize) -> bool {
        debug_assert!(page_idx < iron().total_pages);

        let mem_regions = self.get_mem_region_array_ref_mut();

        if mem_regions[page_idx].is_free.get() {
            return true;
        }

        false
    }
}
