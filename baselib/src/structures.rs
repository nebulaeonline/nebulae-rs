pub mod tree {
    use core::cell::Cell;

    use crate::common::base::*;

    const COLOR_RED: bool = true;
    const COLOR_BLACK: bool = false;
    
    pub struct RBTree<T> {
        pub root: Cell<*mut RBNode<T>>,
    }
    
    #[repr(C)]
    pub struct RBNode<T> {
        pub key: Cell<u128>,
        pub value: Cell<usize>,
        left: Cell<*mut RBNode<T>>,
        right: Cell<*mut RBNode<T>>,
        color: Cell<bool>,
        n: Cell<u128>,
        pub idx: Cell<usize>,
        pub ptr: Cell<*mut T>,
    }
    
    impl<T> RBTree<T> {
        pub fn new() -> Self {
            RBTree {
                root: Cell::new(core::ptr::null_mut()),
            }
        }
    
        pub fn size(&self) -> u128 {
            self.size_node(self.root.get())
        }
    
        fn size_node(&self, node: *mut RBNode<T>) -> u128 {
            if node == core::ptr::null_mut() {
                return 0;
            }
            unsafe {
                (*node).n.get()
            }
        }
    
        pub fn min(&self) -> u128 {
            unsafe { 
                match self.min_node(self.root.get()).as_ref() {
                    Some(node) => (*node).key.get(),
                    None => 0,           
                }
             }
        }
    
        fn min_node(&self, node: *mut RBNode<T>) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if (*node).left.get() == core::ptr::null_mut() {
                    return node;
                }
                self.min_node((*node).left.get())
            }
        }
    
        pub fn max(&self) -> u128 {
            unsafe {
                match self.max_node(self.root.get()).as_ref() {
                    Some(node) => (*node).key.get(),
                    None => 0,
                }
            }
        }

        fn max_node(&self, node: *mut RBNode<T>) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if (*node).right.get() == core::ptr::null_mut() {
                    return node;
                }
                self.max_node((*node).right.get())
            }
        }

        pub fn ceiling(&self, key: u128) -> Option<u128> {
            let node = self._ceiling_node(self.root.get(), key);
            if node == core::ptr::null_mut() {
                return None;
            }
            unsafe {
                Some((*node).key.get())
            }
        }

        pub fn ceiling_node(&self, key: u128) -> Option<*mut RBNode<T>> {
            let node = self._ceiling_node(self.root.get(), key);
            if node == core::ptr::null_mut() {
                return None;
            }
            unsafe {
                Some(node)
            }
        }

        fn _ceiling_node(&self, node: *mut RBNode<T>, key: u128) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if key == (*node).key.get() {
                    return node;
                }
                if key > (*node).key.get() {
                    return self._ceiling_node((*node).right.get(), key);
                }
                let left = self._ceiling_node((*node).left.get(), key);
                if left != core::ptr::null_mut() {
                    return left;
                }
                return node;
            }
        }

        pub fn floor(&self, key: u128) -> Option<u128> {
            let node = self.floor_node(self.root.get(), key);
            if node == core::ptr::null_mut() {
                return None;
            }
            unsafe {
                Some((*node).key.get())
            }
        }
    
        fn floor_node(&self, node: *mut RBNode<T>, key: u128) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if key == (*node).key.get() {
                    return node;
                }
                if key < (*node).key.get() {
                    return self.floor_node((*node).left.get(), key);
                }
                let right = self.floor_node((*node).right.get(), key);
                if right != core::ptr::null_mut() {
                    return right;
                }
                return node;
            }
        }
    
        pub fn select(&self, k: u128) -> Option<u128> {
            let node = self.select_node(self.root.get(), k);
            if node == core::ptr::null_mut() {
                return None;
            }
            unsafe {
                Some((*node).key.get())
            }
        }
    
        fn select_node(&self, node: *mut RBNode<T>, k: u128) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                let t = self.size_node((*node).left.get());
                if t > k {
                    return self.select_node((*node).left.get(), k);
                } else if t < k {
                    return self.select_node((*node).right.get(), k - t - 1);
                } else {
                    return node;
                }
            }
        }
    
        pub fn rank(&self, key: u128) -> u128 {
            self.rank_node(self.root.get(), key)
        }
    
        fn rank_node(&self, node: *mut RBNode<T>, key: u128) -> u128 {
            if node == core::ptr::null_mut() {
                return 0;
            }
            unsafe {
                if key < (*node).key.get() {
                    return self.rank_node((*node).left.get(), key);
                } else if key > (*node).key.get() {
                    return 1 + self.size_node((*node).left.get()) + self.rank_node((*node).right.get(), key);
                } else {
                    return self.size_node((*node).left.get());
                }
            }
        }
    
        pub fn put(&self, node: *mut RBNode<T>) {
            self.root.set(self.put_node(self.root.get(), node));
            unsafe {
                (*self.root.get()).color.set(COLOR_BLACK);
            }
        }
    
        fn put_node(&self, mut node: *mut RBNode<T>, new_node: *mut RBNode<T>) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return new_node;
            }
            unsafe {
                if (*new_node).key.get() < (*node).key.get() {
                    (*node).left.set(self.put_node((*node).left.get(), new_node));
                } else if (*new_node).key.get() > (*node).key.get() {
                    (*node).right.set(self.put_node((*node).right.get(), new_node));
                } else {
                    (*node).value.set((*new_node).value.get());
                }
                if self.is_red((*node).right.get()) && !self.is_red((*node).left.get()) {
                    node = self.rotate_left(node);
                }
                if self.is_red((*node).left.get()) && self.is_red((*node).left.get()) {
                    node = self.rotate_right(node);
                }
                if self.is_red((*node).left.get()) && self.is_red((*node).right.get()) {
                    self.flip_colors(node);
                }
                (*node).n.set(1 + self.size_node((*node).left.get()) + self.size_node((*node).right.get()));
                return node;
            }
        }
    
        fn is_red(&self, node: *mut RBNode<T>) -> bool {
            if node == core::ptr::null_mut() {
                return false;
            }
            unsafe {
                (*node).color.get() == COLOR_RED
            }
        }
    
        fn rotate_left(&self, node: *mut RBNode<T>) -> *mut RBNode<T> {
            unsafe {
                let x = (*node).right.get();
                (*node).right.set((*x).left.get());
                (*x).left.set(node);
                (*x).color.set((*node).color.get());
                (*node).color.set(COLOR_RED);
                (*x).n.set((*node).n.get());
                (*node).n.set(1 + self.size_node((*node).left.get()) + self.size_node((*node).right.get()));
                return x;
            }
        }
    
        fn rotate_right(&self, node: *mut RBNode<T>) -> *mut RBNode<T> {
            unsafe {
                let x = (*node).left.get();
                (*node).left.set((*x).right.get());
                (*x).right.set(node);
                (*x).color.set((*node).color.get());
                (*node).color.set(COLOR_RED);
                (*x).n.set((*node).n.get());
                (*node).n.set(1 + self.size_node((*node).left.get()) + self.size_node((*node).right.get()));
                return x;
            }
        }
    
        fn flip_colors(&self, node: *mut RBNode<T>) {
            unsafe {
                (*node).color.set(COLOR_RED);
                (*(*node).left.get()).color.set(COLOR_BLACK);
                (*(*node).right.get()).color.set(COLOR_BLACK);
            }
        }

        pub fn get(&self, key: u128) -> Option<usize> {
            let node = self.get_node(self.root.get(), key);
            if node == core::ptr::null_mut() {
                return None;
            }
            unsafe {
                Some((*node).value.get())
            }
        }

        fn get_node(&self, node: *mut RBNode<T>, key: u128) -> *mut RBNode<T> {
            if node == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if key < (*node).key.get() {
                    return self.get_node((*node).left.get(), key);
                } else if key > (*node).key.get() {
                    return self.get_node((*node).right.get(), key);
                } else {
                    return node;
                }
            }
        }

        pub fn delete_min(&self) {
            unsafe {
                if !self.is_red((*self.root.get()).left.get()) && !self.is_red((*self.root.get()).right.get()) {
                    (*self.root.get()).color.set(COLOR_RED);
                }
                self.root.set(self.delete_min_node(self.root.get()));
                (*self.root.get()).color.set(COLOR_BLACK);
            }
        }

        unsafe fn delete_min_node(&self, mut node: *mut RBNode<T>) -> *mut RBNode<T> {
            if (*node).left.get() == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if !self.is_red((*node).left.get()) && !self.is_red((*(*node).left.get()).left.get()) {
                    node = self.move_red_left(node);
                }
                (*node).left.set(self.delete_min_node((*node).left.get()));
                return self.balance(node);
            }
        }

        pub fn delete_max(&self) {
            if unsafe { !self.is_red((*self.root.get()).left.get()) && !self.is_red((*self.root.get()).right.get()) } {
                unsafe {
                    (*self.root.get()).color.set(COLOR_RED);
                }
            }
            self.root.set(self.delete_max_node(self.root.get()));
            unsafe {
                (*self.root.get()).color.set(COLOR_BLACK);
            }
        }

        fn delete_max_node(&self, mut node: *mut RBNode<T>) -> *mut RBNode<T> {
            if self.is_red(unsafe { (*node).left.get() }) {
                node = self.rotate_right(node);
            }
            if unsafe { (*node).right.get() } == core::ptr::null_mut() {
                return core::ptr::null_mut();
            }
            unsafe {
                if !self.is_red((*node).right.get()) && !self.is_red((*(*node).right.get()).left.get()) {
                    node = self.move_red_right(node);
                }
                (*node).right.set(self.delete_max_node((*node).right.get()));
                return self.balance(node);
            }
        }

        pub fn delete(&self, key: u128) {
            if unsafe { !self.is_red((*self.root.get()).left.get()) && !self.is_red((*self.root.get()).right.get()) } {
                unsafe {
                    (*self.root.get()).color.set(COLOR_RED);
                }
            }
            self.root.set(self.delete_node(self.root.get(), key));
            unsafe {
                (*self.root.get()).color.set(COLOR_BLACK);
            }
        }

        fn delete_node(&self, mut node: *mut RBNode<T>, key: u128) -> *mut RBNode<T> {
            unsafe {
                if key < (*node).key.get() {
                    if !self.is_red((*node).left.get()) && !self.is_red((*(*node).left.get()).left.get()) {
                        node = self.move_red_left(node);
                    }
                    (*node).left.set(self.delete_node((*node).left.get(), key));
                } else {
                    if self.is_red((*node).left.get()) {
                        node = self.rotate_right(node);
                    }
                    if key == (*node).key.get() && (*node).right.get() == core::ptr::null_mut() {
                        return core::ptr::null_mut();
                    }
                    if !self.is_red((*node).right.get()) && !self.is_red((*(*node).right.get()).left.get()) {
                        node = self.move_red_right(node);
                    }
                    if key == (*node).key.get() {
                        let x = self.min_node((*node).right.get());
                        (*node).key.set((*x).key.get());
                        (*node).value.set((*x).value.get());
                        (*node).right.set(self.delete_min_node((*node).right.get()));
                    } else {
                        (*node).right.set(self.delete_node((*node).right.get(), key));
                    }
                }
            }
            return self.balance(node);
        }

        fn move_red_left(&self, mut node: *mut RBNode<T>) -> *mut RBNode<T> {
            self.flip_colors(node);
            if self.is_red(unsafe { (*(*node).right.get()).left.get() }) {
                unsafe {
                    (*node).right.set(self.rotate_right((*node).right.get()));
                    node = self.rotate_left(node);
                    self.flip_colors(node);
                }
            }
            return node;
        }

        fn move_red_right(&self, mut node: *mut RBNode<T>) -> *mut RBNode<T> {
            self.flip_colors(node);
            if self.is_red(unsafe { (*(*node).left.get()).left.get() }) {
                unsafe {
                    node = self.rotate_right(node);
                    self.flip_colors(node);
                }
            }
            return node;
        }

        fn balance(&self, mut node: *mut RBNode<T>) -> *mut RBNode<T> {
            unsafe {
                if self.is_red((*node).right.get()) {
                    node = self.rotate_left(node);
                }
                if self.is_red((*node).left.get()) && self.is_red((*(*node).left.get()).left.get()) {
                    node = self.rotate_right(node);
                }
                if self.is_red((*node).left.get()) && self.is_red((*node).right.get()) {
                    self.flip_colors(node);
                }
                (*node).n.set(1 + self.size_node((*node).left.get()) + self.size_node((*node).right.get()));
            }
            return node;
        }

        pub fn print(&self) {
            self.print_node(self.root.get());
        }

        fn print_node(&self, node: *mut RBNode<T>) {
            if node == core::ptr::null_mut() {
                return;
            }
            unsafe {
                self.print_node((*node).left.get());
                serial_println!("{} {}", (*node).key.get(), (*node).value.get() as usize);
                self.print_node((*node).right.get());
            }
        }

        pub fn print_tree(&self) {
            self.print_tree_node(self.root.get(), 0);
        }

        fn print_tree_node(&self, node: *mut RBNode<T>, indent: usize) {
            if node == core::ptr::null_mut() {
                return;
            }
            unsafe {
                self.print_tree_node((*node).right.get(), indent + 1);
                for _ in 0..indent {
                    serial_print!("  ");
                }
                serial_println!("{} {}", (*node).key.get(), (*node).value.get() as usize);
                self.print_tree_node((*node).left.get(), indent + 1);
            }
        }

        pub fn print_tree_color(&self) {
            self.print_tree_color_node(self.root.get(), 0);
        }

        fn print_tree_color_node(&self, node: *mut RBNode<T>, indent: usize) {
            if node == core::ptr::null_mut() {
                return;
            }
            unsafe {
                self.print_tree_color_node((*node).right.get(), indent + 1);
                for _ in 0..indent {
                    serial_print!("  ");
                }
                if (*node).color.get() == COLOR_RED {
                    serial_println!("{} {}", (*node).key.get(), (*node).value.get() as usize);
                } else {
                    serial_println!("\x1b[1;30m{} {}\x1b[0m", (*node).key.get(), (*node).value.get() as usize);
                }
                self.print_tree_color_node((*node).left.get(), indent + 1);
            }
        }

        pub fn print_tree_size(&self) {
            self.print_tree_size_node(self.root.get(), 0);
        }

        fn print_tree_size_node(&self, node: *mut RBNode<T>, indent: usize) {
            if node == core::ptr::null_mut() {
                return;
            }
            unsafe {
                self.print_tree_size_node((*node).right.get(), indent + 1);
                for _ in 0..indent {
                    serial_print!("  ");
                }
                serial_println!("{} {}", (*node).key.get(), (*node).value.get() as usize);
                self.print_tree_size_node((*node).left.get(), indent + 1);
            }
        }

        pub fn print_tree_size_color(&self) {
            self.print_tree_size_color_node(self.root.get(), 0);
        }

        fn print_tree_size_color_node(&self, node: *mut RBNode<T>, indent: usize) {
            if node == core::ptr::null_mut() {
                return;
            }
            unsafe {
                self.print_tree_size_color_node((*node).right.get(), indent + 1);
                for _ in 0..indent {
                    serial_print!("  ");
                }
                if (*node).color.get() == COLOR_RED {
                    serial_println!("{} {} {}", (*node).key.get(), (*node).value.get() as usize, (*node).n.get());
                } else {
                    serial_println!("\x1b[1;30m{} {} {}\x1b[0m", (*node).key.get(), (*node).value.get() as usize, (*node).n.get());
                }
                self.print_tree_size_color_node((*node).left.get(), indent + 1);
            }
        }
    }

    impl<T> RBNode<T> {
        pub fn new() -> Self {
            RBNode {
                key: Cell::new(ZERO_U128),
                value: Cell::new(ZERO_USIZE),
                left: Cell::new(core::ptr::null_mut()),
                right: Cell::new(core::ptr::null_mut()),
                color: Cell::new(COLOR_BLACK),
                n: Cell::new(ZERO_U128),
                idx: Cell::new(usize::MAX),
                ptr: Cell::new(core::ptr::null_mut()),
            }
        }
    }
}