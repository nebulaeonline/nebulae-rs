use core::cell::UnsafeCell;

use crate::nebulae::*;
use crate::structures::bitmap::*;
use crate::common::base::*;
use crate::structures::tree::red_black::*;

use crate::vmem::*;

#[repr(C)]
pub struct MemNode<'n> {
    key: UnsafeCell<u128>,
    value: UnsafeCell<usize>,
    left: UnsafeCell<Option<&'n MemNode<'n>>>,
    right: UnsafeCell<Option<&'n MemNode<'n>>>,
    color: UnsafeCell<bool>,
    n: UnsafeCell<u128>,
}

impl<'n> RBNode<'n> for MemNode<'n> {
    fn new() -> Self {
        MemNode {
            key: UnsafeCell::new(ZERO_U128),
            value: UnsafeCell::new(ZERO_USIZE),
            left: UnsafeCell::new(None),
            right: UnsafeCell::new(None),
            color: UnsafeCell::new(false),
            n: UnsafeCell::new(ZERO_U128),
        }
    }

    fn key(&self) -> u128 {
        let k = self.key.get();
        unsafe { (*k).clone() }
    }

    fn set_key(&self, key: u128) {
        let k = unsafe { self.key.get().as_mut().unwrap() };
        *k = Into::into(key);
    }

    fn value(&self) -> usize {
        unsafe { self.value.get().as_ref().unwrap().clone() }
    }

    fn set_value(&self, value: usize) {
        let v = unsafe { self.value.get().as_mut().unwrap() };
        *v = Into::into(value);
    }

    fn left(&self) -> Option<&'n Self> {
        let lresult = unsafe { self.left.get().as_ref().unwrap() };

        if lresult.is_none() {
            return None;
        } else {
            return Some(lresult.unwrap());
        }
    }

    fn set_left(&self, new_left_node_option: Option<&'n Self>) {
        if new_left_node_option.is_some() {
            let l = unsafe { self.left.get().as_mut().unwrap() };
            *l = new_left_node_option;
        }
    }

    fn right(&self) -> Option<&'n Self> {
        let rresult = unsafe { self.right.get().as_ref().unwrap() };

        if rresult.is_none() {
            return None;
        } else {
            return Some(rresult.unwrap());
        }
    }

    #[allow(refining_impl_trait)]
    fn set_right(&self, new_right_node_option: Option<&'n Self>) {
        if new_right_node_option.is_some() {
            let r = unsafe { self.right.get().as_mut().unwrap() };
            *r = new_right_node_option;
        }
    }

    fn color(&self) -> bool {
        unsafe { self.color.get().as_ref().unwrap().clone() }
    }

    fn set_color(&self, color: bool) {
        let c = unsafe { self.color.get().as_mut().unwrap() };
        *c = color;
    }

    fn n(&self) -> u128 {
        unsafe { self.n.get().as_ref().unwrap().clone() }
    }

    fn set_n(&self, new_n: u128) {
        let n = unsafe { self.n.get().as_mut().unwrap() };
        *n = new_n;
    }
}

pub type MemIdx = usize;

impl<'n> MemNode<'n> {
    // this function expects to be called with the root node pointer in the address list
    // i.e. it expects the key in address, size format
    pub fn contains_addr(&self, addr_root: &'n MemNode<'n>, addr: impl MemAddr + AsUsize + Align) -> Option<&'n MemNode> {
        let node = self._contains_addr(addr_root, addr);
        if node.is_none() {
            return None;
        }
        Some(node.unwrap())
    }

    fn _contains_addr(&self, node: &'n MemNode<'n>, addr: impl MemAddr + AsUsize + Align) -> Option<&'n MemNode> {
        let node_key = node.key();
        let node_start_addr = hi64(node_key);
        let node_size = lo64(node_key);

        // just lol
        if addr.as_usize() >= node_start_addr.as_usize() && addr.as_usize() < node_start_addr.as_usize() + node_size.as_usize() {
            return Some(node);
        }
        if addr.as_usize() >= node_start_addr.as_usize() + node_size.as_usize() && node.right().is_some() {
            return self._contains_addr(node.right().unwrap(), addr);
        } else if addr.as_usize() >= node_start_addr.as_usize() + node_size.as_usize() && node.right().is_none() {
            return None;
        }
        if node.left().is_some() {
            let lresult = self._contains_addr(node.left().unwrap(), addr);

            if lresult.is_some() {
                return Some(lresult.unwrap());
            }
        }
        
        None
    }
}

#[repr(C)]
pub struct FrameDescr<'n> {
    pub mem_block: UnsafeCell<MemBlock<PhysAddr>>,    // Memory block this frame represents
    pub flags: UnsafeCell<usize>,                     // Flags for the frame
    pub owner: UnsafeCell<Owner>,                     // Owner of the frame
    pub mem_frame_idx: UnsafeCell<usize>,             // Index of this frame in the array
    pub size_node: UnsafeCell<MemNode<'n>>,           // Node for the size tree
    pub addr_node: UnsafeCell<MemNode<'n>>,           // Node for the address tree
}

impl<'n> FrameDescr<'n> {
    pub fn new() -> Self {
        FrameDescr {
            mem_block: UnsafeCell::new(MemBlock::new_with(PhysAddr(NEBULAE_TEST_PATTERN), ZERO_USIZE)),
            flags: UnsafeCell::new(ZERO_USIZE),
            owner: UnsafeCell::new(Owner::Nobody),
            mem_frame_idx: UnsafeCell::new(ZERO_USIZE),
            size_node: UnsafeCell::new(MemNode::new()),
            addr_node: UnsafeCell::new(MemNode::new()),
        }
    }

    pub fn new_with(
        start_address: PhysAddr,
        size: usize,
        flags: usize,
        owner: Owner,
    ) -> Self {
        FrameDescr {
            mem_block: UnsafeCell::new(MemBlock::new_with(start_address, size)),
            flags: UnsafeCell::new(flags),
            owner: UnsafeCell::new(owner),
            mem_frame_idx: UnsafeCell::new(ZERO_USIZE),
            size_node: UnsafeCell::new(MemNode::new()),
            addr_node: UnsafeCell::new(MemNode::new()),
        }
    }
}

// the tree allocator is not thread safe, so it's instance
// needs to be wrapped in a lock
#[allow(dead_code)]
pub struct TreeAllocator<'n> {
    // The base address
    //pub mem_frame_nodes_phys_base: UnsafeCell<PhysAddr>,
    mem_frame_nodes: UnsafeCell<&'n mut [FrameDescr<'n>]>,

    count: UnsafeCell<usize>,
    capacity: UnsafeCell<usize>,

    rb_size_free: UnsafeCell<RBTree<'n, MemNode<'n>>>,
    rb_addr_free: UnsafeCell<RBTree<'n, MemNode<'n>>>,
    rb_size_alloc: UnsafeCell<RBTree<'n, MemNode<'n>>>,
    rb_addr_alloc: UnsafeCell<RBTree<'n, MemNode<'n>>>,

    dealloc_since_last_coalesce_free_count: UnsafeCell<usize>,
    pub merge_free_dealloc_interval: UnsafeCell<usize>,

    pub frame_node_slot_bitmap: Option<Bitmap>,
}

impl<'n> TreeAllocator<'n> {
    // Update a range of frames (free/alloc, owner) in our frame info structs
    fn update_frame_info_structs (
        &self,
        mem_frame_idx: usize,
        is_free: bool,
    ) -> bool
    {
        // get a reference to the memory frame array, and calc the start/ending frame indexes
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };
        let start_page_idx = pages::addr_to_page_index(mem_frame_array[mem_frame_idx].mem_block.get_mut().base_addr);
        let end_page_idx = pages::usize_to_page_index(mem_frame_array[mem_frame_idx].mem_block.get_mut().base_addr.as_usize() + mem_frame_array[mem_frame_idx].mem_block.get_mut().size - 1);

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::update_page_info_structs(): -> base_addr = 0x{:0x}, size = {}, starting idx = {}, ending idx = {}", mem_frame_array[mem_frame_idx].mem_block.get_mut().base_addr.as_usize(), mem_frame_array[mem_frame_idx].mem_block.get_mut().size, start_page_idx, end_page_idx);

        // local var
        let max_pages = iron().unwrap().get_total_pages();

        // loop through the pages in the range and update the page info structs' status & owner fields
        {
            let mut page_info_struct_lockptr = iron().unwrap().page_info_structs_01.lock_rw_spin();

            if (*page_info_struct_lockptr).is_none() {
                #[cfg(all(debug_assertions, feature = "serialdbg"))]
                serial_println!("TreeAllocator::update_page_info_structs(): -> reference to page info structs is None: base_addr = 0x{:0x}, size = {}", mem_frame_array[mem_frame_idx].mem_block.get_mut().base_addr.as_usize(), mem_frame_array[mem_frame_idx].mem_block.get_mut().size);
                return false;
            }

            // unwrap is safe
            let page_info_structs = (*page_info_struct_lockptr).as_mut().unwrap();
            
            for i in start_page_idx..=end_page_idx {
                if i >= max_pages {
                    break;
                }

                if is_free {
                    page_info_structs[i].status = pages::PageStatus::Free;
                } else {
                    page_info_structs[i].status = pages::PageStatus::Alloc;
                }
            }
        }
        true
    }

    // Merges the free frames in the tree
    // returns the number of merges that were performed
    fn coalesce_free_frames(&mut self) -> Option<usize> {
        let mut merge_count: usize = 0;

        // get a reference to the memory frame array
        let mem_frames = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // get root node from the free by address tree
        let free_addr_root = unsafe { self.rb_addr_free.get().as_ref().unwrap().root() };
        
        // if there's no root, then there's no memory to coalesce
        if free_addr_root.is_none() {
            return None;
        }

        // get the min node (lowest address) from the free address tree
        let free_addr_min_result = unsafe { self.rb_addr_free.get().as_ref().unwrap().min_node() };
        // if there is no min node, but there was a root node, we have corruption
        if free_addr_min_result.is_none() {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("TreeAllocator::coalesce_free_space(): -> memory subsystem error: reference to min node was None from free address root");
            return None;
        }
        
        // set the current node = to the min node (safety checked above)
        let mut free_addr_node_cur = free_addr_min_result.unwrap();

        // iterate & merge
        loop {
            // get the current node's key
            let node_key = free_addr_node_cur.key();

            // get the current node's address
            let node_addr = hi64(node_key);

            // get the current node's size
            let node_size = lo64(node_key);

            // get the current node's memory frame index
            let node_mem_frame_idx = free_addr_node_cur.value();

            // see if there's a next node
            if free_addr_node_cur.right().is_none() {
                // if there is no next node, we're done
                break;
            }

            // get the next node - safe unwrap
            let next_node = free_addr_node_cur.right().unwrap();
            
            // get the next node's address
            let next_node_addr = hi64(next_node.key().clone());

            // get the next node's index
            let next_node_mem_idx = next_node.value().clone();

            // make sure the current node and the next node are adjacent
            if node_addr.as_usize() + node_size.as_usize() == next_node_addr.as_usize() {
                // they are adjacent, so merge them
                let merged_frame_result = self.merge_free_frames(node_mem_frame_idx, next_node_mem_idx);

                // make sure the merge was successful
                if merged_frame_result.is_none() {
                    #[cfg(all(debug_assertions, feature = "serialdbg"))]
                    serial_println!("TreeAllocator::coalesce_free_frames(): -> error in memory subsystem: failed to merge free frames: {} & {}", node_mem_frame_idx, next_node_mem_idx);
                    return Some(merge_count);
                }

                // update the current node (safe unwrap)
                let free_addr_node_cur_result = unsafe { mem_frames[merged_frame_result.unwrap()].addr_node.get().as_ref().unwrap() }.left();
                if free_addr_node_cur_result.is_none() {
                    return Some(merge_count);
                }
                free_addr_node_cur = free_addr_node_cur_result.unwrap();

                // increment our merge count
                merge_count += 1;

            } else {
                // they are not adjacent, so move to the next node if there is one
                if free_addr_node_cur.right().is_none() {
                    break;
                }
                free_addr_node_cur = free_addr_node_cur.right().unwrap();
            }
        }
        
        // reset the "deallocs since last coalesce" count
        let dslcc = unsafe { self.dealloc_since_last_coalesce_free_count.get().as_mut().unwrap() };
        *dslcc = ZERO_USIZE;

        Some(merge_count)
    }

    // Add a new mem frame to the tree
    pub fn add_mem_frame(
        &mut self,
        base_addr: PhysAddr,
        size: usize,
        is_free: bool,
        flags: usize,
        owner: Owner,
    ) -> Option<usize> {
        
        // get a reference to the memory frame array (should never fail)
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // reserve a slot in the memory frame array
        let new_frame_slot_alloc_result = self.alloc_internal_frame_slot();
        match new_frame_slot_alloc_result {
            Some(new_frame_idx) => {
                
                let mem_block = unsafe { mem_frame_array[new_frame_idx].mem_block.get().as_mut().unwrap() };

                mem_block.base_addr = base_addr;
                mem_block.size = size;
                
                {
                    let flags_ref = unsafe { mem_frame_array[new_frame_idx].flags.get().as_mut().unwrap() };
                    (*flags_ref) = flags;
                }
                
                {
                    let owner_ref = unsafe { mem_frame_array[new_frame_idx].owner.get().as_mut().unwrap() };
                    (*owner_ref) = owner;
                }
                
                {
                    let frame_idx_ref = unsafe { mem_frame_array[new_frame_idx].mem_frame_idx.get().as_mut().unwrap() };
                    (*frame_idx_ref) = new_frame_idx;
                }

                if is_free {
                    self.put_frame_into_free_trunks(new_frame_idx);
                } else {
                    self.put_frame_into_alloc_trunks(new_frame_idx);
                }

                // increment our frame count
                {
                    let updated_frame_count = unsafe { self.count.get().as_ref().unwrap().clone() + 1 };
                    let frame_count_ref = unsafe { self.count.get().as_mut().unwrap() };
                    (*frame_count_ref) = updated_frame_count;
                }

                #[cfg(all(debug_assertions, feature = "serialdbg"))]
                serial_println!("TreeAllocator::add_mem_frame(): -> idx = {}, mem_frame_ptr = 0x{:08x}, base_addr = 0x{:08x}, size = {}; mem frame count: {}", new_frame_idx, &mem_frame_array[new_frame_idx] as *const FrameDescr as usize, base_addr, size, unsafe { self.count.get().as_ref().unwrap() }.clone());
                
                // page info range update should be performed when adding
                // a new frame to the tree
                self.update_frame_info_structs(new_frame_idx, is_free);

                Some(new_frame_idx)
            }
            None => None,
        }
    }

    // completely remove a frame from the allocator
    pub fn remove_frame(&mut self, frame_idx: usize) {
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        if unsafe { mem_frame_array[frame_idx].owner.get().as_ref().unwrap().clone() } == Owner::Nobody {
            self.remove_frame_from_free_trunks(frame_idx);
        } else {
            self.remove_frame_from_alloc_trunks(frame_idx);
        }

        // deallocate the frame slot we were using via the bitmap
        self.dealloc_internal_frame_slot(frame_idx);

        // decrement our frame count
        {
            let updated_frame_count = unsafe { self.count.get().as_ref().unwrap().clone() - 1 };
            let frame_count_ref = unsafe { self.count.get().as_mut().unwrap() };
            (*frame_count_ref) = updated_frame_count;
        }
    }

    // alloc before doing anything else
    fn alloc_internal_frame_slot(&mut self) -> Option<usize> {
        
        // make sure we have a bitmap
        if self.frame_node_slot_bitmap.is_none() {
            
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("TreeAllocator::alloc_internal_frame_slot(): -> bitmap is None, unable to alloc, so returning None");
            return None;
        }

        // get a new slot from the bitmap (unwrap is safe)
        let new_struct_slot = self.frame_node_slot_bitmap.as_mut().unwrap().find_first_set();

        // do the diddly
        match new_struct_slot {
            Some(slot_idx) => {
                self.frame_node_slot_bitmap.as_mut().unwrap().clear(slot_idx); // safe unwrap
                Some(slot_idx)
            }
            None => {
                #[cfg(all(debug_assertions, feature = "serialdbg"))]
                serial_println!("TreeAllocator::alloc_internal_frame_slot(): -> unable to obtain free slot for memory region storage; memory resources may be running low");
                None
            }
        }
    }

    // dealloc after everything else
    fn dealloc_internal_frame_slot(&mut self, frame_idx: usize) -> bool {
        
        // make sure we have a bitmap
        if self.frame_node_slot_bitmap.is_none() {
            return false;
        }

        // make sure we're not out of bounds
        if frame_idx >= unsafe { self.capacity.get().as_ref().unwrap().clone() } {
            return false;
        }

        // do the diddly on the dealloc side
        self.frame_node_slot_bitmap.as_mut().unwrap().set(frame_idx);
        true
    }

    

    // split a frame into two frames: a left and a right--
    // as on a number line, the left frame is to the left of the right frame
    // in terms of addresses (i.e. start_1 < end_1 < start_2 < end_2)
    // this fn returns the left and the right frames or None if the frame 
    // could not be split
    fn split_free_frame(
        &mut self,
        frame_idx: usize,
        left_size: usize,
    ) -> Option<(usize, usize)> {

        // get our aligned size for the new left frame
        let left_aligned_size = align_up(left_size, MEMORY_DEFAULT_PAGE_USIZE);
        let left_frame_idx = frame_idx;

        // get a reference to the memory frame array
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // get the existing stats
        let frame_size = mem_frame_array[left_frame_idx].mem_block.get_mut().size;
        let orig_frame_start_addr = mem_frame_array[left_frame_idx].mem_block.get_mut().base_addr;
        let new_right_aligned_size = frame_size - left_aligned_size;

        // debug output
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::split_free_frame(): -> idx = {}, size = {}, start_addr = 0x{:0x}", left_frame_idx, frame_size, orig_frame_start_addr);
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::split_free_frame(): -> left_aligned_size = {}, right_aligned_size = {}", left_aligned_size, new_right_aligned_size);

        // see if the block is large enough to split
        if frame_size - left_aligned_size > 0 {
           
            // there will be left over space, so split the block
            
            // obtain an index for the new frame into the memory frame array;
            // the memory frame array is controlled by the bitmap
            let new_right_frame = self.alloc_internal_frame_slot();

            // make sure we were able to allocate a new frame slot
            match new_right_frame {
                Some(right_frame_idx) => {
                    
                    // get some stats
                    let new_right_frame_size = frame_size - left_aligned_size;
                    let new_right_frame_start_addr = orig_frame_start_addr.as_usize() + left_aligned_size;

                    // remove the left frame from each tree, update the key/value, 
                    // and add it back to each tree
                    self.remove_frame_from_free_trunks(left_frame_idx);

                    // update the left frame entry
                    mem_frame_array[left_frame_idx].mem_block.get_mut().size = left_aligned_size;
                    
                    {   
                        let owner_ref = unsafe { mem_frame_array[left_frame_idx].owner.get().as_mut().unwrap() };
                        (*owner_ref) = Owner::Nobody;
                    }

                    {
                        let mem_frame_ref = unsafe { mem_frame_array[left_frame_idx].mem_frame_idx.get().as_mut().unwrap() };
                        (*mem_frame_ref) = left_frame_idx;
                    }

                    // update the left frame's node keys & values
                    {
                        // size node
                        let size_node = unsafe { mem_frame_array[left_frame_idx].size_node.get().as_ref().unwrap() };
                        size_node.set_value(left_frame_idx);
                        size_node.set_key(make128(left_aligned_size, orig_frame_start_addr.as_usize()));
                    }

                    {
                        // addr node
                        let addr_node = unsafe { mem_frame_array[left_frame_idx].addr_node.get().as_ref().unwrap() };
                        addr_node.set_value(left_frame_idx);
                        addr_node.set_key(make128(orig_frame_start_addr.as_usize(), left_aligned_size));
                    }

                    // update the left frame's page entries
                    self.update_frame_info_structs(
                        left_frame_idx,
                        false,
                    );

                    // add the left frame back to each tree trunk
                    self.put_frame_into_free_trunks(left_frame_idx);

                    // update the newly created right frame's entry
                    mem_frame_array[right_frame_idx]
                        .mem_block.get_mut().base_addr = PhysAddr(new_right_frame_start_addr);

                    mem_frame_array[right_frame_idx].mem_block.get_mut().size = new_right_frame_size;
                    
                    {
                        let owner_ref = unsafe { mem_frame_array[right_frame_idx].owner.get().as_mut().unwrap() };
                        (*owner_ref) = Owner::Nobody;
                    }

                    {
                        let frame_ref = unsafe { mem_frame_array[right_frame_idx].mem_frame_idx.get().as_mut().unwrap() };
                        (*frame_ref) = right_frame_idx;
                    }

                    // set the right frame's node keys & values
                    {
                        // size node
                        let size_node = unsafe { mem_frame_array[right_frame_idx].size_node.get().as_ref().unwrap() };
                        size_node.set_value(right_frame_idx);
                        size_node.set_key(make128(new_right_frame_size, new_right_frame_start_addr.as_usize()));
                    }

                    {
                        // addr node
                        let addr_node = unsafe { mem_frame_array[right_frame_idx].addr_node.get().as_ref().unwrap() };
                        addr_node.set_value(right_frame_idx);
                        addr_node.set_key(make128(new_right_frame_start_addr.as_usize(), new_right_frame_size));
                    }
                    
                    // update the right frame's page entries
                    self.update_frame_info_structs(
                        right_frame_idx,
                        true,
                    );

                    // add the new right frame to the tree trunks
                    self.put_frame_into_free_trunks(right_frame_idx);

                    // we created a new frame, so increment our count
                    {
                        let updated_frame_count = unsafe { self.count.get().as_ref().unwrap().clone() + 1 };
                        let frame_count_ref = unsafe { self.count.get().as_mut().unwrap() };
                        (*frame_count_ref) = updated_frame_count;
                    }

                    Some((left_frame_idx, right_frame_idx))
                }
                None => None,
            }
        } else if frame_size == left_aligned_size {
            // return none since the frame does not need to be split
            None
        }
        else {
            // return none since the frame is too small to split
            None
        }
    }

    // remove a frame from the free trunks
    fn remove_frame_from_free_trunks(&mut self, frame_idx: usize) {
        debug_assert!(frame_idx < unsafe { self.capacity.get().as_ref().unwrap().clone() });

        let mem_frames = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        {
            let size_node = unsafe { mem_frames[frame_idx].size_node.get().as_ref().unwrap() };
            let size_trunk = unsafe { self.rb_size_free.get().as_ref().unwrap() };
            size_trunk.delete(size_node.key());
        }
        
        {
            let addr_node = unsafe { mem_frames[frame_idx].addr_node.get().as_ref().unwrap() }; 
            let addr_trunk = unsafe { self.rb_addr_free.get().as_ref().unwrap() };
            addr_trunk.delete(addr_node.key());
        }        
    }

    // remove a frame from the alloc'ed trunks
    #[allow(dead_code)]
    fn remove_frame_from_alloc_trunks(&mut self, frame_idx: usize) {
        debug_assert!(frame_idx < unsafe { self.capacity.get().as_ref().unwrap().clone() });

        let mem_frames = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        {
            let size_node = unsafe { mem_frames[frame_idx].size_node.get().as_ref().unwrap() };
            let size_trunk = unsafe { self.rb_size_alloc.get().as_ref().unwrap() };
            size_trunk.delete(size_node.key());
        }
        
        {
            let addr_node = unsafe { mem_frames[frame_idx].addr_node.get().as_ref().unwrap() };
            let addr_trunk = unsafe { self.rb_addr_alloc.get().as_ref().unwrap() };
            addr_trunk.delete(addr_node.key());
        }        
    }

    // put a frame into the free trunks
    fn put_frame_into_free_trunks(&mut self, frame_idx: usize) {
        debug_assert!(frame_idx < unsafe { self.capacity.get().as_ref().unwrap().clone() });

        let mem_frames = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        let frame_size = mem_frames[frame_idx].mem_block.get_mut().size;
        let frame_start_addr = mem_frames[frame_idx].mem_block.get_mut().base_addr;

        {
            let size_node = unsafe { mem_frames[frame_idx].size_node.get().as_ref().unwrap() };
            
            size_node.set_value(frame_idx);
            size_node.set_key(make128(frame_size, frame_start_addr.as_usize()));
            
            let size_trunk = self.rb_size_free.get_mut();
            size_trunk.put(&*size_node);
        }

        {
            let addr_node = unsafe { mem_frames[frame_idx].addr_node.get().as_ref().unwrap() };

            addr_node.set_value(frame_idx);
            addr_node.set_key(make128(frame_start_addr.as_usize(), frame_size));
            
            let addr_trunk = self.rb_addr_free.get_mut();
            addr_trunk.put(&*addr_node);
        }
    }

    // put a frame into the alloc'ed trunks
    #[allow(dead_code)]
    fn put_frame_into_alloc_trunks(&mut self, frame_idx: usize) {
        debug_assert!(frame_idx < unsafe { self.capacity.get().as_ref().unwrap().clone() });

        let mem_frames = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };
        let frame_size = mem_frames[frame_idx].mem_block.get_mut().size;
        let frame_start_addr = mem_frames[frame_idx].mem_block.get_mut().base_addr;

        
        {
            let size_node = unsafe { mem_frames[frame_idx].size_node.get().as_ref().unwrap() };
            size_node.set_value(frame_idx);
            size_node.set_key(make128(frame_size, frame_start_addr.as_usize()));

            let size_trunk = unsafe { self.rb_size_alloc.get().as_mut().unwrap() };
            size_trunk.put(&*size_node);
        }

        {
            let addr_node = unsafe { mem_frames[frame_idx].addr_node.get().as_ref().unwrap() };
            addr_node.set_value(frame_idx);
            addr_node.set_key(make128(frame_start_addr.as_usize(), frame_size));

            let addr_trunk = unsafe { self.rb_addr_alloc.get().as_mut().unwrap() };
            addr_trunk.put(&*addr_node);
        }
    }

    // merge two free frames
    fn merge_free_frames(&mut self, left_frame_idx: usize, right_frame_idx: usize) -> Option<usize> {
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // make sure the frames are free
        if !(unsafe { mem_frame_array[left_frame_idx].owner.get().as_ref().unwrap() }.clone() == Owner::Nobody) || !(unsafe { mem_frame_array[right_frame_idx].owner.get().as_ref().unwrap() }.clone() == Owner::Nobody) {
            return None;
        }

        // make sure the frames are adjacent
        if mem_frame_array[left_frame_idx].mem_block.get_mut().base_addr.as_usize() + mem_frame_array[left_frame_idx].mem_block.get_mut().size != mem_frame_array[right_frame_idx].mem_block.get_mut().base_addr.as_usize() {
            return None;
        }

        // remove the frames from each trunk
        self.remove_frame_from_free_trunks(left_frame_idx);
        self.remove_frame_from_free_trunks(right_frame_idx);

        // update the left frame entry (this will be the new merged frame)
        let new_frame_size = mem_frame_array[left_frame_idx].mem_block.get_mut().size + mem_frame_array[right_frame_idx].mem_block.get_mut().size;

        mem_frame_array[left_frame_idx].mem_block.get_mut().size = new_frame_size;
        
        {
            let owner_ref = unsafe { mem_frame_array[left_frame_idx].owner.get().as_mut().unwrap() };
            (*owner_ref) = Owner::Nobody;
        }

        {
            let frame_ref = unsafe { mem_frame_array[left_frame_idx].mem_frame_idx.get().as_mut().unwrap() };
            (*frame_ref) = left_frame_idx;
        }

        // temp var
        let frame_start_addr = mem_frame_array[left_frame_idx].mem_block.get_mut().base_addr.clone();

        // update the info on the left frame's nodes
        {
            let size_node = unsafe { mem_frame_array[left_frame_idx].size_node.get().as_mut().unwrap() };
            size_node.set_value(left_frame_idx);
            size_node.set_key(make128(new_frame_size, frame_start_addr.as_usize()));
        }

        {
            let addr_node = unsafe { mem_frame_array[left_frame_idx].addr_node.get().as_mut().unwrap() };
            addr_node.set_value(left_frame_idx);
            addr_node.set_key(make128(frame_start_addr.as_usize(), new_frame_size));
        }
        
        // update the left frame's info structs
        self.update_frame_info_structs(
            left_frame_idx,
            true,
        );

        // update the right frame's info structs
        self.update_frame_info_structs(
            right_frame_idx,
            true,
        );

        // add the left frame back into the free trunks
        self.put_frame_into_free_trunks(left_frame_idx);

        // remove the right frame entirely
        self.remove_frame(right_frame_idx);

        // return the left frame's index
        Some(left_frame_idx)
    }    
    
    // mark a frame as allocated
    pub fn mark_frame_allocated(&mut self, frame_idx: usize, owner: Owner) {
        
        // obtain a reference to the memory frame array
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // make sure we're not out of bounds
        if frame_idx >= unsafe { self.capacity.get().as_ref().unwrap().clone() } {
            panic!("frame_idx out of bounds");
        }

        // see if the frame is already allocated
        // if it is, then we don't need to do anything
        if !(unsafe { mem_frame_array[frame_idx].owner.get().as_ref().unwrap() }.clone() == Owner::Nobody) {
            return;
        }

        // stats
        let frame_size = mem_frame_array[frame_idx].mem_block.get_mut().size;
        let frame_start_addr = mem_frame_array[frame_idx].mem_block.get_mut().base_addr;

        {
            let owner_ref = unsafe { mem_frame_array[frame_idx].owner.get().as_mut().unwrap() };
            (*owner_ref) = owner;
        }

        // always remove the frame from the trees before making
        // any changes to the frame's keys, otherwise the rb tree
        // will be corrupted

        // remove the frame from the free trunks
        self.remove_frame_from_free_trunks(frame_idx);

        // update key/value for size trunk node
        {
            let size_node = unsafe { mem_frame_array[frame_idx].size_node.get().as_ref().unwrap() };
            size_node.set_value(frame_idx);
            size_node.set_key(make128(frame_size, frame_start_addr.as_usize()));   
        }

        // update key/value for address trunk node
        {
            let addr_node = unsafe { mem_frame_array[frame_idx].addr_node.get().as_ref().unwrap() };
            addr_node.set_value(frame_idx);
            addr_node.set_key(make128(frame_start_addr.as_usize(), frame_size));           
        }
        
        // update the page info structs
        self.update_frame_info_structs(frame_idx, false);

        // add the frame to the alloc trunks
        self.put_frame_into_alloc_trunks(frame_idx);
    }

    // mark a frame as free
    pub fn mark_frame_free(&mut self, frame_idx: usize, owner: Owner) -> bool {
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // make sure we're not out of bounds
        if frame_idx >= unsafe { self.capacity.get().as_ref().unwrap().clone() } {
            return false;
        }

        // make sure the owner is correct
        if unsafe { mem_frame_array[frame_idx].owner.get().as_ref().unwrap() }.clone() != owner {
            return false;
        }
        
        // see if the frame is already free
        // if it is, then we don't need to do anything
        if unsafe { mem_frame_array[frame_idx].owner.get().as_ref().unwrap() }.clone() == Owner::Nobody {
            return false;
        }

        // stats
        
        let frame_size = mem_frame_array[frame_idx].mem_block.get_mut().size;
        let frame_start_addr = mem_frame_array[frame_idx].mem_block.get_mut().base_addr;

        {
            let owner_ref = unsafe { mem_frame_array[frame_idx].owner.get().as_mut().unwrap() };
            (*owner_ref) = Owner::Nobody;
        }

        // always remove the frame from the trees before making
        // any changes to the frame's keys, otherwise the rb tree
        // will be corrupted

        // remove the frame from the alloc trunks
        self.remove_frame_from_alloc_trunks(frame_idx);

        // update key/value for size trunk node
        {
            let size_node = unsafe { mem_frame_array[frame_idx].size_node.get().as_ref().unwrap() };
            size_node.set_value(frame_idx);
            size_node.set_key(make128(frame_size, frame_start_addr.as_usize()));            
        }

        // update key/value for address trunk node
        {
            let addr_node = unsafe { mem_frame_array[frame_idx].addr_node.get().as_ref().unwrap() };
            addr_node.set_value(frame_idx);
            addr_node.set_key(make128(frame_start_addr.as_usize(), frame_size));            
        }
        
        // update the page info structs
        self.update_frame_info_structs(frame_idx, true);

        // add the frame to the free trunks
        self.put_frame_into_free_trunks(frame_idx);

        true
    }
}

impl<'n> FrameAllocator for TreeAllocator<'n> {
    fn new(mem_nodes_base: PhysAddr, node_count: usize) -> Self {
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::new(): -> allocating a new TreeAllocator");

        let ret = TreeAllocator {
            mem_frame_nodes: UnsafeCell::new(
                unsafe {
                    core::slice::from_raw_parts_mut::<'n, FrameDescr>(
                        raw::abracadabra_ptr_mut::<FrameDescr, PhysAddr>(mem_nodes_base, false),
                        node_count,
                    )
                }
            ),
            count: UnsafeCell::new(ZERO_USIZE),
            capacity: UnsafeCell::new(ZERO_USIZE),

            rb_size_free: UnsafeCell::new(RBTree::<MemNode>::new()),
            rb_addr_free: UnsafeCell::new(RBTree::<MemNode>::new()),
            rb_size_alloc: UnsafeCell::new(RBTree::<MemNode>::new()),
            rb_addr_alloc: UnsafeCell::new(RBTree::<MemNode>::new()),

            dealloc_since_last_coalesce_free_count: UnsafeCell::new(0usize),
            merge_free_dealloc_interval: UnsafeCell::new(FRAME_ALLOCATOR_COALESCE_THRESHOLD_DEALLOC),

            frame_node_slot_bitmap: Some(Bitmap::new(Owner::Memory)),            
        };

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::new(): -> allocation complete");

        ret
    }

    // ok to panic in frame allocator init
    fn init(&mut self) {

        let neb = iron().unwrap();

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::init(): -> iron: nebulae struct @ {:#x}", neb as *const Nebulae as usize);

        let total_frames = neb.get_total_pages();
        {
            let cap_ref = unsafe { self.capacity.get().as_mut().unwrap() };
            (*cap_ref) = total_frames;
        }

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::init(): -> allocator capacity set to {} frames; init complete", total_frames);
    }

    // Allocates memory by physical address & size
    fn alloc_frame_fixed(&mut self, phys_addr: PhysAddr, size: usize, page_size: PageSize, owner: Owner) -> Option<PhysAddr> {
        
        debug_assert!(phys_addr.is_aligned(page_size.as_usize()));

        // align the address to the page size
        let aligned_size = size.align_up(page_size.as_usize());

        // quick stats
        let aligned_size_in_pages = pages::bytes_to_pages(aligned_size, page_size);
        let aligned_size_in_bytes = pages::pages_to_bytes(aligned_size_in_pages, page_size);

        // on a fixed allocation, we need to coalesce the free blocks
        self.coalesce_free_frames();
        
        // We need to see if the frame containing the desired address is free

        // get a reference to the root node of the address tree's free trunk
        let addr_free_root_mem_node_result = 
            self.rb_addr_free
                .get_mut()
                .root();

        // if we don't have a root node right now, that means we have no free memory.
        // don't panic, just hope no one notices.
        if addr_free_root_mem_node_result.is_none() {
            return None;
        }

        // get references to the root node of the address tree's free trunk
        let addr_free_root_mem_node_ref = addr_free_root_mem_node_result.unwrap() as &MemNode;
        let addr_free_root_mem_node = addr_free_root_mem_node_result.unwrap();
        
        // Find the memory node that contains the address
        let parent_node_result = addr_free_root_mem_node_ref.contains_addr(
            addr_free_root_mem_node,
            phys_addr,
        );

        // see if we got anything
        if parent_node_result.is_none() {
            return None;
        }

        // we just verified the result is not None and not null, so we can safely unwrap
        let mem_frame_node = parent_node_result.unwrap();
        let parent_node_frame_idx = mem_frame_node.value();

        // Now we need to know if this frame is large enough for the proposed allocation
        
        // get a link to the memory frame array
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };
        
        // get the stats of the memory frame
        let frame_start_addr = mem_frame_array[parent_node_frame_idx].mem_block.get_mut().base_addr;
        let frame_size = mem_frame_array[parent_node_frame_idx].mem_block.get_mut().size;
        
        // calculate the offset of the desired address from the base address of the frame
        let addr_split_offset = phys_addr.as_usize() - frame_start_addr.as_usize();

        // see if the frame is too small to contain the desired address
        if frame_size - addr_split_offset < aligned_size_in_bytes {
            // the frame, even after coalescing free space, is not large enough
            // we must fail the allocation
            return None;
        }
        else {
            // if there's no offset we check then split.
            // if there is an offset, we always split, then check for a 2nd round.

            // we know the frame is large enough at this point
            if addr_split_offset == 0 {
                // there is no offset, so we can allocate the frame directly
              
                // see if we need to split the frame
                if frame_size - aligned_size_in_bytes >= MEMORY_DEFAULT_PAGE_USIZE {
                    // there is excess capacity, so split the block
                    // this time, we will allocate the left block
                    //let (left_node_idx, _right_node_idx) = 
                    let try_split_result = 
                        self.split_free_frame(
                            parent_node_frame_idx, 
                            aligned_size_in_bytes);

                    // make sure the split was successful
                    if try_split_result.is_none() {
                        return None;
                    }
                    
                    // unwrap is safe
                    let (left_node_idx, _right_node_idx) = try_split_result.unwrap();
                    
                    // mark the frame as allocated
                    self.mark_frame_allocated(left_node_idx, owner);

                    // zero the new block
                    raw::memset_aligned(
                        mem_frame_array[left_node_idx].mem_block.get_mut().base_addr,
                        aligned_size,
                        0usize,
                    );
                } else {
                    // the frame and the allocation request are the same size;
                    // mark the frame as allocated
                    self.mark_frame_allocated(parent_node_frame_idx, owner);

                    // zero the new block
                    raw::memset_aligned(
                        mem_frame_array[parent_node_frame_idx].mem_block.get_mut().base_addr,
                        aligned_size,
                        0usize,
                    );
                }
            } else {
                // get a new frame slot
                let new_frame_slot_result = self.alloc_internal_frame_slot();

                // make sure we obtained a new frame slot
                if new_frame_slot_result.is_none() {
                    return None;
                }

                // split @ the offset creating a new frame (the offset frame will be left, allocated frame will be right)
                let try_split_result =
                    self.split_free_frame(parent_node_frame_idx, addr_split_offset);

                // make sure the split was successful
                if try_split_result.is_none() {
                    return None;
                }

                // unwrap is safe
                let (_left_node_idx, right_node_idx) = try_split_result.unwrap();
                
                // get the stats of the new frame
                let new_frame_size = mem_frame_array[right_node_idx].mem_block.get_mut().size;

                // see if the new frame is large enough to split
                if new_frame_size - aligned_size_in_bytes >= MEMORY_DEFAULT_PAGE_USIZE {
                    // there is excess capacity, so split the block.
                    // this time, we will allocate the left block
                    let try_split_result_2 =
                        self.split_free_frame(
                            right_node_idx, 
                            aligned_size_in_bytes);

                    // make sure the split was successful
                    if try_split_result_2.is_none() {
                        return None;
                    }

                    // unwrap is safe
                    let (left_node_idx_2, _right_node_idx_2) = try_split_result_2.unwrap();
                    
                    // mark the frame as allocated
                    self.mark_frame_allocated(left_node_idx_2, owner);

                    // zero the new block
                    raw::memset_aligned(
                        mem_frame_array[left_node_idx_2].mem_block.get_mut().base_addr,
                        aligned_size,
                        0usize,
                    );
                } else {
                    // mark the frame as allocated
                    self.mark_frame_allocated(parent_node_frame_idx, owner);

                    // zero the new block
                    raw::memset_aligned(
                        mem_frame_array[parent_node_frame_idx].mem_block.get_mut().base_addr,
                        aligned_size,
                        0usize,
                    );
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
            self.coalesce_free_frames();
        }

        let mem_frames = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };
        let aligned_size = align_up(size, page_size.as_usize());

        // best fit
        let size_key = make128(aligned_size, 0);
        let mut block_idx: usize;
        let mut comp_node = unsafe { self.rb_size_free.get().as_mut().unwrap().ceiling_node(size_key) };

        if comp_node.is_none() {
            // no free blocks large enough to satisfy the request
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("TreeAllocator::alloc_frame(): no free blocks large enough to satisfy the request -> size = {}, size_key = 0x{:0x}", aligned_size, size_key);
            
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            unsafe { self.rb_size_free.get().as_ref().unwrap().print_tree() };

            return None;
        }

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::alloc_frame(): size = {}, size_key = 0x{:0x}, comp_node = 0x{:0x}", 
            aligned_size, size_key, comp_node.unwrap() as *const MemNode as usize);

        while comp_node.is_some() {
            // unwraps are safe; comp_node is not none
            let addr = lo64(comp_node.unwrap().key()) as usize;
            let sz = hi64(comp_node.unwrap().key()) as usize;
            block_idx = comp_node.unwrap().value();

            if addr.is_aligned(page_size.as_usize()) && sz >= aligned_size {
                
                if sz == aligned_size {
                    // if the page size is exact, then we don't need to split

                    // mark the block as allocated
                    self.mark_frame_allocated(block_idx, owner);

                    // zero the new block
                    raw::memset_aligned(
                        mem_frames[block_idx].mem_block.get_mut().base_addr,
                        aligned_size,
                        ZERO_USIZE,
                    );

                    return Some(mem_frames[block_idx].mem_block.get_mut().base_addr);

                } else {

                    // split the block
                    let nodes_opt =
                        self.split_free_frame(block_idx, aligned_size);

                    match nodes_opt {
                        Some((left_node_idx, _right_node_idx)) => {
                            self.mark_frame_allocated(left_node_idx, owner);

                            // zero the new block
                            raw::memset_aligned(
                                mem_frames[left_node_idx].mem_block.get_mut().base_addr,
                                aligned_size,
                                0usize,
                            );
                            return Some(mem_frames[left_node_idx].mem_block.get_mut().base_addr);
                        }
                        None => {
                            #[cfg(all(debug_assertions, feature = "serialdbg"))]
                            serial_println!("TreeAllocator::alloc_frame(): -> could not split memory frame (left alloc)");
                            return None;
                        }
                    }
                }              
            } else {
                // see what it would take to align this frame
                let aligned_addr = align_up(addr, page_size.as_usize());

                if aligned_addr + aligned_size <= addr + sz {
                    // this frame can be aligned
                    
                    // split the block
                    // the first split will take off the lower addresses to bring the base address of
                    // the frame up to the alignment boundary
                    let nodes_opt =
                        self.split_free_frame(block_idx, aligned_addr - addr);

                    match nodes_opt {

                        // the left frame will be what we trimmed to meet alignment requirements
                        // so we're only interested in the right frame
                        Some((_, right_node_idx)) => {
                            
                            // see if we need to split the block again
                            if mem_frames[right_node_idx].mem_block.get_mut().size - aligned_size >= MEMORY_DEFAULT_PAGE_USIZE {
                                
                                let nodes_opt2 =
                                    self.split_free_frame(right_node_idx, aligned_size);

                                match nodes_opt2 {
                                    Some((left_node_idx2, _)) => {
                                        self.mark_frame_allocated(left_node_idx2, owner);

                                        // zero the new block
                                        raw::memset_aligned(
                                            mem_frames[left_node_idx2].mem_block.get_mut().base_addr,
                                            aligned_size,
                                            ZERO_USIZE,
                                        );
                                        
                                        return Some(mem_frames[left_node_idx2].mem_block.get_mut().base_addr);
                                    }
                                    None => {
                                        #[cfg(all(debug_assertions, feature = "serialdbg"))]
                                        serial_println!("TreeAllocator::alloc_frame(): -> could not re-split memory frame (old alloc)");
                                        return None;
                                    }
                                }
                            } else {
                                // the block is too small to split, but it will work
                                // mark the frame as allocated
                                self.mark_frame_allocated(right_node_idx, owner);

                                // zero the new block
                                raw::memset_aligned(
                                    mem_frames[right_node_idx].mem_block.get_mut().base_addr,
                                    aligned_size,
                                    0usize,
                                );

                                return Some(mem_frames[right_node_idx].mem_block.get_mut().base_addr);
                            }
                        }
                        None => {
                            #[cfg(all(debug_assertions, feature = "serialdbg"))]
                            serial_println!("TreeAllocator::alloc_frame(): -> could not split memory frame (new alloc)");
                            return None;
                        }
                    }
                }
            }

            // move to the next comparison node
            // unwrap is safe with '?' operator
            unsafe {
                comp_node = self
                    .rb_size_free
                    .get().as_ref().unwrap()
                    .ceiling_node(comp_node.unwrap().key() + 1);
            }
        }
        None
    }

    // Deallocates a single page of memory of the specified size
    // page_base is the base address of the page to deallocate, not
    // the base address of the frame that contains the page
    fn dealloc_frame(&mut self, page_base: PhysAddr, owner: Owner) -> bool {
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("TreeAllocator::dealloc_page(): page_base = 0x{:0x}", page_base.as_usize());

        // get a reference to the memory frame array
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        // get the index of the frame that contains the page
        //let frame_idx = pages::addr_to_page_index(page_base);

        // get a reference to the root node of the address tree alloc trunk
        let addr_alloc_root_mem_node_result = 
            unsafe {
                self.rb_addr_alloc
                    .get().as_mut().unwrap()
                    .root()
            };

        // if there is no root node in the alloc address tree, we haven't
        // allocated any memory, so we can just return
        if addr_alloc_root_mem_node_result.is_none() {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("TreeAllocator::dealloc_page(): no memory to deallocate");
            return false;
        }

        // unwrap our ref (twice) - this is safe in this context-
        // we are only doing this to facilitate the contains_addr() call below
        let addr_alloc_root_mem_node_ref = addr_alloc_root_mem_node_result.unwrap() as &MemNode;
        let addr_alloc_root_mem_node = addr_alloc_root_mem_node_result.unwrap();

        // Find the memory node that contains the address
        let mem_node_result = addr_alloc_root_mem_node_ref.contains_addr(
            addr_alloc_root_mem_node,
            page_base,
        );

        // see if we got any results
        if mem_node_result.is_none() {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("TreeAllocator::dealloc_page(): no memory to deallocate");
            return false;
        }

        // we just verified the result is not None, so we can unwrap
        let mem_node = mem_node_result.unwrap();
        let frame_to_dealloc_idx = mem_node.value();

        // make sure the owners match
        if unsafe { mem_frame_array[frame_to_dealloc_idx].owner.get().as_ref().unwrap() }.clone() != owner {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("TreeAllocator::dealloc_frame(): -> owner mismatch: {:?} tried to deallocate physical memory owned by {:?}", owner, unsafe { mem_frame_array[frame_to_dealloc_idx].owner.get().as_ref().unwrap().clone() });
            return false;
        }

        // mark the frame as free
        self.mark_frame_free(frame_to_dealloc_idx, owner);

        // see if we need to coalesce the free space
        if unsafe { self.dealloc_since_last_coalesce_free_count.get().as_ref().unwrap() }.clone() > unsafe { self.merge_free_dealloc_interval.get().as_ref().unwrap() }.clone() {
            self.coalesce_free_frames();
            {
                let count_ref = unsafe { self.dealloc_since_last_coalesce_free_count.get().as_mut().unwrap() };
                (*count_ref) = ZERO_USIZE;
            }
        } else {
            let new_count = unsafe { self.dealloc_since_last_coalesce_free_count.get().as_ref().unwrap().clone() + 1 };
            {
                let count_ref = unsafe { self.dealloc_since_last_coalesce_free_count.get().as_mut().unwrap() };
                (*count_ref) = new_count;
            }
        }

        // update the page info structs
        self.update_frame_info_structs(frame_to_dealloc_idx, true);
        true
    }

    fn free_page_count(&mut self) -> usize {
        self.free_mem_count() / MEMORY_DEFAULT_PAGE_USIZE
    }

    fn free_mem_count(&mut self) -> usize {
        self.rb_size_free.get_mut().sum_upper() as usize
    }

    fn total_page_count(&self) -> usize {
        iron().unwrap().get_total_pages()
    }

    fn total_mem_count(&self) -> usize {
        iron().unwrap().get_phys_mem_boundary().as_usize()
    }

    fn is_memory_frame_free(&self, page_base: PhysAddr) -> bool {
        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };
        let page_index = pages::addr_to_page_index(page_base);

        debug_assert!(page_index < iron().unwrap().get_total_pages());

        if unsafe { mem_frame_array[page_index].owner.get().as_ref().unwrap() }.clone() == Owner::Nobody {
            return true;
        }

        false
    }

    fn is_frame_index_free(&self, page_idx: usize) -> bool {
        debug_assert!(page_idx < iron().unwrap().get_total_pages());

        let mem_frame_array = unsafe { self.mem_frame_nodes.get().as_mut().unwrap() };

        if unsafe { mem_frame_array[page_idx].owner.get().as_ref().unwrap() }.clone() == Owner::Nobody {
            return true;
        }

        false
    }
}
