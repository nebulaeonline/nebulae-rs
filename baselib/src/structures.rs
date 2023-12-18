pub mod tree {

    pub mod red_black {

        use crate::common::base::*;

        const COLOR_RED: bool = true;
        const COLOR_BLACK: bool = false;

        pub trait RBNode {
            fn new() -> Self;

            fn key(&self) -> u128;
            fn value(&self) -> usize;
            fn left(&self) -> *mut Self;
            fn right(&self) -> *mut Self;
            fn color(&self) -> bool;
            fn n(&self) -> u128;
            fn idx(&self) -> usize;
            fn ptr(&self) -> *mut ();

            fn set_key(&self, key: u128);
            fn set_value(&self, value: usize);
            fn set_left(&self, left: *mut Self);
            fn set_right(&self, right: *mut Self);
            fn set_color(&self, color: bool);
            fn set_n(&self, n: u128);
            fn set_idx(&self, idx: usize);
            fn set_ptr(&self, ptr: *mut ());
        }

        pub struct RBTree<T: RBNode> {
            root: *mut T,
        }

        impl<T> RBTree<T>
        where
            T: RBNode,
        {
            pub fn root(&self) -> *mut T {
                self.root
            }

            pub fn set_root(&self, root: *mut T) {
                self.root = root;
            }

            pub fn new() -> Self {
                Self {
                    root: core::ptr::null_mut(),
                }
            }

            pub fn size(&self) -> u128 {
                self.size_node(self.root())
            }

            pub fn size_node(&self, node: *mut T) -> u128 {
                if node == core::ptr::null_mut() {
                    return 0;
                }
                unsafe { (*node).n() }
            }

            pub fn min(&self) -> u128 {
                unsafe {
                    match self.min_node(self.root()).as_ref() {
                        Some(node) => (*node).key(),
                        None => 0,
                    }
                }
            }

            pub fn min_node(&self, node: *mut T) -> *mut T {
                if node == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if (*node).left() == core::ptr::null_mut() {
                        return node;
                    }
                    self.min_node((*node).left())
                }
            }

            pub fn max(&self) -> u128 {
                unsafe {
                    match self.max_node(self.root()).as_ref() {
                        Some(node) => (*node).key(),
                        None => 0,
                    }
                }
            }

            pub fn max_node(&self, node: *mut T) -> *mut T {
                if node == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if (*node).right() == core::ptr::null_mut() {
                        return node;
                    }
                    self.max_node((*node).right())
                }
            }

            pub fn ceiling(&self, key: u128) -> Option<u128> {
                let node = self._ceiling_node(self.root(), key);
                if node == core::ptr::null_mut() {
                    return None;
                }
                unsafe { Some((*node).key()) }
            }

            pub fn ceiling_node(&self, key: u128) -> Option<*mut T> {
                let node = self._ceiling_node(self.root(), key);
                if node == core::ptr::null_mut() {
                    return None;
                }
                unsafe { Some(node) }
            }

            fn _ceiling_node(&self, node: *mut T, key: u128) -> *mut T {
                if node == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if key == (*node).key() {
                        return node;
                    }
                    if key > (*node).key() {
                        return self._ceiling_node((*node).right(), key);
                    }
                    let left = self._ceiling_node((*node).left(), key);
                    if left != core::ptr::null_mut() {
                        return left;
                    }
                    return node;
                }
            }

            pub fn floor(&self, key: u128) -> Option<u128> {
                let node = self.floor_node(self.root(), key);
                if node == core::ptr::null_mut() {
                    return None;
                }
                unsafe { Some((*node).key()) }
            }

            pub fn floor_node(&self, node: *mut T, key: u128) -> *mut T {
                if node == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if key == (*node).key() {
                        return node;
                    }
                    if key < (*node).key() {
                        return self.floor_node((*node).left(), key);
                    }
                    let right = self.floor_node((*node).right(), key);
                    if right != core::ptr::null_mut() {
                        return right;
                    }
                    return node;
                }
            }

            pub fn select(&self, k: u128) -> Option<u128> {
                let node = self.select_node(self.root(), k);
                if node == core::ptr::null_mut() {
                    return None;
                }
                unsafe { Some((*node).key()) }
            }

            pub fn select_node(&self, node: *mut T, k: u128) -> *mut T {
                if node == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    let t = self.size_node((*node).left());
                    if t > k {
                        return self.select_node((*node).left(), k);
                    } else if t < k {
                        return self.select_node((*node).right(), k - t - 1);
                    } else {
                        return node;
                    }
                }
            }

            pub fn rank(&self, key: u128) -> u128 {
                self.rank_node(self.root(), key)
            }

            pub fn rank_node(&self, node: *mut T, key: u128) -> u128 {
                if node == core::ptr::null_mut() {
                    return 0;
                }
                unsafe {
                    if key < (*node).key() {
                        return self.rank_node((*node).left(), key);
                    } else if key > (*node).key() {
                        return 1
                            + self.size_node((*node).left())
                            + self.rank_node((*node).right(), key);
                    } else {
                        return self.size_node((*node).left());
                    }
                }
            }

            pub fn put(&self, node: *mut T) {
                self.set_root(self.put_node(self.root(), node));
                unsafe {
                    (*self.root()).set_color(COLOR_BLACK);
                }
            }

            fn put_node(&self, mut node: *mut T, new_node: *mut T) -> *mut T {
                if node == core::ptr::null_mut() {
                    return new_node;
                }
                unsafe {
                    if (*new_node).key() < (*node).key() {
                        (*node).set_left(self.put_node((*node).left(), new_node));
                    } else if (*new_node).key() > (*node).key() {
                        (*node).set_right(self.put_node((*node).right(), new_node));
                    } else {
                        (*node).set_value((*new_node).value());
                    }
                    if self.is_red((*node).right()) && !self.is_red((*node).left()) {
                        node = self.rotate_left(node);
                    }
                    if self.is_red((*node).left()) && self.is_red((*node).left()) {
                        node = self.rotate_right(node);
                    }
                    if self.is_red((*node).left()) && self.is_red((*node).right()) {
                        self.flip_colors(node);
                    }
                    (*node).set_n(
                        1 + self.size_node((*node).left()) + self.size_node((*node).right()),
                    );
                    return node;
                }
            }

            fn is_red(&self, node: *mut T) -> bool {
                if node == core::ptr::null_mut() {
                    return false;
                }
                unsafe { (*node).color() == COLOR_RED }
            }

            fn rotate_left(&self, node: *mut T) -> *mut T {
                unsafe {
                    let x = (*node).right();
                    (*node).set_right((*x).left());
                    (*x).set_left(node);
                    (*x).set_color((*node).color());
                    (*node).set_color(COLOR_RED);
                    (*x).set_n((*node).n());
                    (*node).set_n(
                        1 + self.size_node((*node).left()) + self.size_node((*node).right()),
                    );
                    return x;
                }
            }

            fn rotate_right(&self, node: *mut T) -> *mut T {
                unsafe {
                    let x = (*node).left();
                    (*node).set_left((*x).right());
                    (*x).set_right(node);
                    (*x).set_color((*node).color());
                    (*node).set_color(COLOR_RED);
                    (*x).set_n((*node).n());
                    (*node).set_n(
                        1 + self.size_node((*node).left()) + self.size_node((*node).right()),
                    );
                    return x;
                }
            }

            fn flip_colors(&self, node: *mut T) {
                unsafe {
                    (*node).set_color(COLOR_RED);
                    (*(*node).left()).set_color(COLOR_BLACK);
                    (*(*node).right()).set_color(COLOR_BLACK);
                }
            }

            pub fn get(&self, key: u128) -> Option<usize> {
                let node = self.get_node(self.root(), key);
                if node == core::ptr::null_mut() {
                    return None;
                }
                unsafe { Some((*node).value()) }
            }

            pub fn get_node(&self, node: *mut T, key: u128) -> *mut T {
                if node == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if key < (*node).key() {
                        return self.get_node((*node).left(), key);
                    } else if key > (*node).key() {
                        return self.get_node((*node).right(), key);
                    } else {
                        return node;
                    }
                }
            }

            pub fn delete_min(&self) {
                unsafe {
                    if !self.is_red((*self.root()).left()) && !self.is_red((*self.root()).right()) {
                        (*self.root()).set_color(COLOR_RED);
                    }
                    self.set_root(self.delete_min_node(self.root()));
                    (*self.root()).set_color(COLOR_BLACK);
                }
            }

            unsafe fn delete_min_node(&self, mut node: *mut T) -> *mut T {
                if (*node).left() == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if !self.is_red((*node).left()) && !self.is_red((*(*node).left()).left()) {
                        node = self.move_red_left(node);
                    }
                    (*node).set_left(self.delete_min_node((*node).left()));
                    return self.balance(node);
                }
            }

            pub fn delete_max(&self) {
                if unsafe {
                    !self.is_red((*self.root()).left()) && !self.is_red((*self.root()).right())
                } {
                    unsafe {
                        (*self.root()).set_color(COLOR_RED);
                    }
                }
                self.set_root(self.delete_max_node(self.root()));
                unsafe {
                    (*self.root()).set_color(COLOR_BLACK);
                }
            }

            fn delete_max_node(&self, mut node: *mut T) -> *mut T {
                if self.is_red(unsafe { (*node).left() }) {
                    node = self.rotate_right(node);
                }
                if unsafe { (*node).right() } == core::ptr::null_mut() {
                    return core::ptr::null_mut();
                }
                unsafe {
                    if !self.is_red((*node).right()) && !self.is_red((*(*node).right()).left()) {
                        node = self.move_red_right(node);
                    }
                    (*node).set_right(self.delete_max_node((*node).right()));
                    return self.balance(node);
                }
            }

            pub fn delete(&self, key: u128) {
                if unsafe {
                    !self.is_red((*self.root()).left()) && !self.is_red((*self.root()).right())
                } {
                    unsafe {
                        (*self.root()).set_color(COLOR_RED);
                    }
                }
                self.set_root(self.delete_node(self.root(), key));
                unsafe {
                    (*self.root()).set_color(COLOR_BLACK);
                }
            }

            fn delete_node(&self, mut node: *mut T, key: u128) -> *mut T {
                unsafe {
                    if key < (*node).key() {
                        if !self.is_red((*node).left()) && !self.is_red((*(*node).left()).left()) {
                            node = self.move_red_left(node);
                        }
                        (*node).set_left(self.delete_node((*node).left(), key));
                    } else {
                        if self.is_red((*node).left()) {
                            node = self.rotate_right(node);
                        }
                        if key == (*node).key() && (*node).right() == core::ptr::null_mut() {
                            return core::ptr::null_mut();
                        }
                        if !self.is_red((*node).right()) && !self.is_red((*(*node).right()).left())
                        {
                            node = self.move_red_right(node);
                        }
                        if key == (*node).key() {
                            let x = self.min_node((*node).right());
                            (*node).set_key((*x).key());
                            (*node).set_value((*x).value());
                            (*node).set_right(self.delete_min_node((*node).right()));
                        } else {
                            (*node).set_right(self.delete_node((*node).right(), key));
                        }
                    }
                }
                return self.balance(node);
            }

            fn move_red_left(&self, mut node: *mut T) -> *mut T {
                self.flip_colors(node);
                if self.is_red(unsafe { (*(*node).right()).left() }) {
                    unsafe {
                        (*node).set_right(self.rotate_right((*node).right()));
                        node = self.rotate_left(node);
                        self.flip_colors(node);
                    }
                }
                return node;
            }

            fn move_red_right(&self, mut node: *mut T) -> *mut T {
                self.flip_colors(node);
                if self.is_red(unsafe { (*(*node).left()).left() }) {
                    unsafe {
                        node = self.rotate_right(node);
                        self.flip_colors(node);
                    }
                }
                return node;
            }

            fn balance(&self, mut node: *mut T) -> *mut T {
                unsafe {
                    if self.is_red((*node).right()) {
                        node = self.rotate_left(node);
                    }
                    if self.is_red((*node).left()) && self.is_red((*(*node).left()).left()) {
                        node = self.rotate_right(node);
                    }
                    if self.is_red((*node).left()) && self.is_red((*node).right()) {
                        self.flip_colors(node);
                    }
                    (*node).set_n(
                        1 + self.size_node((*node).left()) + self.size_node((*node).right()),
                    );
                }
                return node;
            }

            pub fn print(&self) {
                self.print_node(self.root());
            }

            pub fn print_node(&self, node: *mut T) {
                if node == core::ptr::null_mut() {
                    return;
                }
                unsafe {
                    self.print_node((*node).left());
                    serial_println!("{} {}", (*node).key(), (*node).value() as usize);
                    self.print_node((*node).right());
                }
            }

            pub fn print_tree(&self) {
                self.print_tree_node(self.root(), 0);
            }

            pub fn print_tree_node(&self, node: *mut T, indent: usize) {
                if node == core::ptr::null_mut() {
                    return;
                }
                unsafe {
                    self.print_tree_node((*node).right(), indent + 1);
                    for _ in 0..indent {
                        serial_print!("  ");
                    }
                    serial_println!("{} {}", (*node).key(), (*node).value() as usize);
                    self.print_tree_node((*node).left(), indent + 1);
                }
            }

            pub fn print_tree_color(&self) {
                self.print_tree_color_node(self.root(), 0);
            }

            pub fn print_tree_color_node(&self, node: *mut T, indent: usize) {
                if node == core::ptr::null_mut() {
                    return;
                }
                unsafe {
                    self.print_tree_color_node((*node).right(), indent + 1);
                    for _ in 0..indent {
                        serial_print!("  ");
                    }
                    if (*node).color() == COLOR_RED {
                        serial_println!("{} {}", (*node).key(), (*node).value() as usize);
                    } else {
                        serial_println!(
                            "\x1b[1;30m{} {}\x1b[0m",
                            (*node).key(),
                            (*node).value() as usize
                        );
                    }
                    self.print_tree_color_node((*node).left(), indent + 1);
                }
            }

            pub fn print_tree_size(&self) {
                self.print_tree_size_node(self.root(), 0);
            }

            pub fn print_tree_size_node(&self, node: *mut T, indent: usize) {
                if node == core::ptr::null_mut() {
                    return;
                }
                unsafe {
                    self.print_tree_size_node((*node).right(), indent + 1);
                    for _ in 0..indent {
                        serial_print!("  ");
                    }
                    serial_println!("{} {}", (*node).key(), (*node).value() as usize);
                    self.print_tree_size_node((*node).left(), indent + 1);
                }
            }

            pub fn print_tree_size_color(&self) {
                self.print_tree_size_color_node(self.root(), 0);
            }

            pub fn print_tree_size_color_node(&self, node: *mut T, indent: usize) {
                if node == core::ptr::null_mut() {
                    return;
                }
                unsafe {
                    self.print_tree_size_color_node((*node).right(), indent + 1);
                    for _ in 0..indent {
                        serial_print!("  ");
                    }
                    if (*node).color() == COLOR_RED {
                        serial_println!(
                            "{} {} {}",
                            (*node).key(),
                            (*node).value() as usize,
                            (*node).n()
                        );
                    } else {
                        serial_println!(
                            "\x1b[1;30m{} {} {}\x1b[0m",
                            (*node).key(),
                            (*node).value() as usize,
                            (*node).n()
                        );
                    }
                    self.print_tree_size_color_node((*node).left(), indent + 1);
                }
            }
        }
    }
}
