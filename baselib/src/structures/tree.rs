pub mod red_black {

    use crate::common::base::*;
    use core::cell::UnsafeCell;

    const COLOR_RED: bool = true;
    const COLOR_BLACK: bool = false;

    pub trait RBNode<'n> {
        fn new() -> Self;

        fn key(&self) -> u128;
        fn value(&self) -> usize;
        fn left(&self) -> Option<&'n Self>;
        fn right(&self) -> Option<&'n Self>;
        fn color(&self) -> bool;
        fn n(&self) -> u128;

        fn set_key(&self, key: u128);
        fn set_value(&self, value: usize);
        fn set_left(&self, left: Option<&'n Self>);
        fn set_right(&self, right: Option<&'n Self>);
        fn set_color(&self, color: bool);
        fn set_n(&self, n: u128);
    }

    pub struct RBTree<'n, T: RBNode<'n>> {
        root: UnsafeCell<Option<&'n T>>,
    }

    impl<'n, T> RBTree<'n, T>
    where
        T: RBNode<'n>,
    {
        // some unsafe
        pub fn root(&self) -> Option<&'n T> {
            if unsafe { self.root.get().as_ref().unwrap().is_none() } {
                return None;
            }
            Some(unsafe { self.root.get().as_ref().unwrap().unwrap() })
        }

        // unsafe
        pub fn set_root(&self, root: &'n T) {
            unsafe { self.root.get().as_mut().unwrap().replace(root) };
        }

        // no unsafe
        pub fn new() -> Self {
            Self {
                root: UnsafeCell::new(None),
            }
        }

        // some unsafe
        pub fn size(&self) -> Option<u128> {
            let root_node_result = self.root.get();

            if root_node_result.is_null() {
                return None;
            }

            match unsafe { *root_node_result } {
                None => None,
                Some(node) => {
                    Some(self.node_n(node))
                }
            }
        }

        // no unsafe
        pub fn node_n(&self, node: &T) -> u128 {
            node.n()
        }

        // no unsafe
        pub fn min(&self) -> Option<u128> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._min_node(root_node) {
                        Some(node) => Some(node.key()),
                        None => None,
                    }
                }
            }
        }

        // no unsafe
        pub fn min_node(&self) -> Option<&'n T> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._min_node(root_node) {
                        Some(node) => Some(node),
                        None => None,
                    }
                }
            }   
        }
        
        // no unsafe
        pub fn _min_node(&self, node: &'n T) -> Option<&'n T> {
            if node.left().is_none() {
                return Some(node);
            }
            self._min_node(node.left().unwrap())
        }

        // no unsafe
        pub fn max(&self) -> Option<u128> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._max_node(root_node) {
                        Some(node) => Some(node.key()),
                        None => None,
                    }
                }
            }
        }

        // no unsafe
        pub fn max_node(&self) -> Option<&T> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._max_node(root_node) {
                        Some(node) => Some(node),
                        None => None,
                    }
                }
            }
        }

        // no unsafe
        pub fn _max_node(&self, node: &'n T) -> Option<&'n T> {
            if node.right().is_none() {
                return Some(node);
            }
            self._max_node(node.right().unwrap())
        }

        // no unsafe
        pub fn ceiling(&self, key: u128) -> Option<u128> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._ceiling_node(root_node, key) {
                        Some(node) => Some(node.key()),
                        None => None,
                    }
                }
            }
        }

        // no unsafe
        pub fn ceiling_node(&self, key: u128) -> Option<&T> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._ceiling_node(root_node, key) {
                        Some(node) => Some(node),
                        None => None,
                    }
                }
            }
        }

        // no unsafe
        fn _ceiling_node(&self, node: &'n T, key: u128) -> Option<&'n T> {
            // the key is equal to our key
            if key == node.key() {
                return Some(node);
            }
            
            // the key is greater than us
            if key > node.key() {
                // if it's greater than us, and we don't have a right subtree, then no nodes in the tree will fulfill the request
                if node.right().is_none() {
                    return None;
                }
                return self._ceiling_node(node.right().unwrap(), key);
            }
            
            // the key is less than us, but we need to check our left subtree first
            if node.left().is_some() {
                let lresult = self._ceiling_node(node.left().unwrap(), key);
                
                // return whatever node bubbles up from the left subtree
                if lresult.is_some() {
                    return lresult;
                }
            }

            // the key is less than us, but we don't have a left subtree (or already processed it), so we are the ceiling
            return Some(node);
        }

        // no unsafe
        pub fn floor(&self, key: u128) -> Option<u128> {
            let root_node_result = self.floor_node(key);

            match root_node_result {
                None => None,
                Some(floor_node) => {
                    Some(floor_node.key())
                }
            }
        }

        // no unsafe
        pub fn floor_node(&self, key: u128) -> Option<&T> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self._floor_node(root_node, key) {
                        Some(node) => Some(node),
                        None => None,
                    }
                }
            }
        }

        // no unsafe
        pub fn _floor_node(&self, node: &'n T, key: u128) -> Option<&'n T> {
            
            // the key is equal to our key
            if key == node.key() {
                return Some(node);
            }

            // the key is less than us
            if key < node.key() {
                if node.left().is_none() {
                    // if the key is less then us and we have no left subtree, then no nodes in the tree will fulfill the request
                    return None;
                }
                return self._floor_node(node.left().unwrap(), key);
            }

            // the key is greater than us, so we need to check our right subtree
            if node.right().is_some() {
                let rresult = self._floor_node(node.right().unwrap(), key);

                if rresult.is_some() {
                    return rresult;
                }
            }
            
            // the key is greater than us, but we don't have a right subtree, so we are the floor
            return Some(node);
        }

        // no unsafe
        pub fn select(&self, k: u128) -> Option<u128> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self.select_node(root_node, k) {
                        Some(node) => Some(node.key()),
                        None => None,
                    }
                },
            }
        }

        // no unsafe
        pub fn select_node(&self, node: &'n T, k: u128) -> Option<&'n T> {
            let t = {
                if node.left().is_some() {
                    self.node_n(node.left().unwrap())
                } else {
                    0
                }
            };
            if t > k {
                if node.left().is_some() {
                    return self.select_node(node.left().unwrap(), k);
                } else {
                    return None;
                }
            } else if t < k {
                if node.right().is_some() {
                    return self.select_node(node.right().unwrap(), k - t - 1);
                } else {
                    return None;
                }
            } else {
                return Some(node);
            }
        }

        // no unsafe
        pub fn rank(&self, key: u128) -> Option<u128> {
            let root_node_result = self.root();

            match root_node_result {
                None => None,
                Some(root_node) => {
                    match self.rank_node(root_node, key) {
                        Some(rank) => Some(rank),
                        None => None,
                    }
                },
            }
        }

        // no unsafe
        pub fn rank_node(&self, node: &T, key: u128) -> Option<u128> {
            if key < node.key() {
                if node.left().is_none() {
                    return None;
                }
                return self.rank_node(node.left().unwrap(), key);
            } else if key > node.key() {
                return Some(
                    1
                    + if node.left().is_some() { self.node_n(node.left().unwrap()) } else { ZERO_U128 }
                    + if node.right().is_some() { self.rank_node(node.right().unwrap(), key).unwrap() } else { ZERO_U128 });
            } else {
                if node.left().is_none() {
                    return Some(0);
                }
                return Some(self.node_n(node.left().unwrap()));
            }
        }

        // no unsafe
        pub fn sum_upper(&self) -> u64 {
            let root_node_result = self.root();

            match root_node_result {
                None => ZERO_U64,
                Some(root_node) => {
                    match self._sum(root_node, true) {
                        Some(sum) => sum,
                        None => ZERO_U64,
                    }
                },
            }
        }

        // no unsafe
        pub fn sum_lower(&self) -> u64 {
            let root_node_result = self.root();

            match root_node_result {
                None => ZERO_U64,
                Some(root_node) => {
                    match self._sum(root_node, false) {
                        Some(sum) => sum,
                        None => ZERO_U64,
                    }
                },
            }
        }

        // no unsafe
        fn _sum(&self, node: &'n T, upper: bool) -> Option<u64> {
            // we only care about the upper bits of the key
            let addend = if upper {
                hi64(node.key())
            } else {
                lo64(node.key())
            };

            return Some(
                addend 
                + if node.left().is_some() { self._sum(node.left().unwrap(), upper).unwrap() } else { ZERO_U64 }
                + if node.right().is_some() { self._sum(node.right().unwrap(), upper).unwrap() } else { ZERO_U64 });
        }

        // no unsafe
        pub fn put(&self, node: &'n T) -> bool {
            let root_node_result = self.root();

            match root_node_result {
                None => {
                    self.set_root(node);
                    self.root().unwrap().set_color(COLOR_BLACK);
                },
                Some(root_node) => {
                    let put_result = self.put_node(root_node, node);
                    put_result.set_color(COLOR_BLACK);
                    
                    self.set_root(put_result);
                },
            }
            true
        }

        // no unsafe
        fn put_node(&self, mut node: &'n T, new_node: &'n T) -> &'n T {
            if new_node.key() < node.key() {
                if node.left().is_some() {
                    node.set_left(Some(self.put_node(node.left().unwrap(), new_node)));
                } else {
                    node.set_left(Some(new_node));
                }
            } else if new_node.key() > node.key() {
                if node.right().is_some() {
                    node.set_right(Some(self.put_node(node.right().unwrap(), new_node)));
                } else {
                    node.set_right(Some(new_node));
                }
            } else {
                node.set_value(new_node.value());
            }

            if (node.right().is_some() && self.is_red(node.right().unwrap())) && (node.left().is_some() && !self.is_red(node.left().unwrap())) {
                node = self.rotate_left(node);
            }

            if (node.left().is_some() && self.is_red(node.left().unwrap())) && (node.left().is_some() && self.is_red(node.left().unwrap())) {
                node = self.rotate_right(node);
            }

            if (node.left().is_some() && self.is_red(node.left().unwrap())) && (node.right().is_some() && self.is_red(node.right().unwrap())) {
                self.flip_colors(node);
            }
            
            node.set_n(
                1 
                + if node.left().is_some() { self.node_n(node.left().unwrap()) } else { 0 }
                + if node.right().is_some() { self.node_n(node.right().unwrap()) } else { 0 },
            );
            return node;

        }

        // no unsafe
        fn is_red(&self, node: &'n T) -> bool {
            node.color() == COLOR_RED
        }

        
        // no unsafe
        fn rotate_left(&self, node: &'n T) -> &'n T {
            debug_assert!(node.right().is_some());

            let x = node.right().unwrap();
            if x.left().is_some() {
                node.set_right(Some(x.left().unwrap()));
            }
            x.set_left(Some(node));
            x.set_color(node.color());
            node.set_color(COLOR_RED);
            x.set_n(node.n());
            node.set_n(
                1 
                + if node.left().is_some() { self.node_n(node.left().unwrap()) } else { ZERO_U128 }
                + if node.right().is_some() { self.node_n(node.right().unwrap()) } else { ZERO_U128 },
            );
            return x;
        }

        // no unsafe
        fn rotate_right(&self, node: &'n T) -> &'n T {
            debug_assert!(node.left().is_some());

            let x = node.left().unwrap();
            if x.right().is_some() {
                node.set_left(Some(x.right().unwrap()));
            }
            x.set_right(Some(node));
            x.set_color(node.color());
            node.set_color(COLOR_RED);
            x.set_n(node.n());
            node.set_n(
                1 
                + if node.left().is_some() { self.node_n(node.left().unwrap()) } else { ZERO_U128 }
                + if node.right().is_some() { self.node_n(node.right().unwrap()) } else { ZERO_U128 },
            );
            return x;
        }

        // no unsafe
        fn flip_colors(&self, node: &'n T) {
            node.set_color(COLOR_RED);

            if node.left().is_some() { (node.left().unwrap()).set_color(COLOR_BLACK); }
            if node.right().is_some() { (node.right().unwrap()).set_color(COLOR_BLACK); }
        }

        // no unsafe
        pub fn get(&self, key: u128) -> Option<usize> {
            let root_node = self.root()?;
            let node = self.get_node(root_node, key);
            if node.is_none() {
                return None;
            }
            Some(node.unwrap().value())
        }

        // no unsafe
        pub fn get_node(&self, node: &'n T, key: u128) -> Option<&'n T> {
            if key < node.key() {
                if node.left().is_some() {
                    return self.get_node(node.left().unwrap(), key);
                } else {
                    return None;
                }
            } else if key > node.key() {
                if node.right().is_some() {
                    return self.get_node(node.right().unwrap(), key);
                } else {
                    return None;
                }
            } else {
                return Some(node);
            }
        }

        // no unsafe
        pub fn delete(&self, key: u128) -> bool {
            if self.root().is_none() {
                return false;
            }

            if (self.root().unwrap().left().is_some() && !self.is_red(self.root().unwrap().left().unwrap())) && 
               (self.root().unwrap().right().is_some() && !self.is_red(self.root().unwrap().right().unwrap())) {

                self.root().unwrap().set_color(COLOR_RED);
            }

            let dresult = self.delete_node(self.root().unwrap(), key);
            if dresult.is_none() {
                return false;
            }

            self.set_root(dresult.unwrap());
            self.root().unwrap().set_color(COLOR_BLACK);
            true
        }

        // no unsafe
        fn _delete_min_node(&self, mut node: &'n T) -> Option<&'n T> {
            if node.left().is_none() {
                return None;
            }

            if (node.left().is_some() && !self.is_red(node.left().unwrap())) && 
               (node.left().unwrap().left().is_some() && !self.is_red(node.left().unwrap().left().unwrap())) {

                node = self.move_red_left(node);
            }

            let dresult = self._delete_min_node(node.left().unwrap());
            if dresult.is_some() {
                node.set_left(Some(dresult.unwrap()));
            } else {
                node.set_left(None);
            }

            Some(self.balance(node))
        }

        // no unsafe
        fn delete_node(&self, mut node: &'n T, key: u128) -> Option<&'n T> {
            if key < node.key() {
                if (node.left().is_some() && !self.is_red(node.left().unwrap())) && 
                    (node.left().unwrap().left().is_some() && !self.is_red((node.left().unwrap()).left().unwrap())) {

                    node = self.move_red_left(node);
                }

                if node.left().is_some() { 
                    let dresult = self.delete_node(node.left().unwrap(), key);
                    if dresult.is_some() {
                        node.set_left(Some(dresult.unwrap()));
                    }
                }
            } else {
                if node.left().is_some() && self.is_red(node.left().unwrap()) {
                    node = self.rotate_right(node);
                }
                if key == node.key() && node.right().is_none() {
                    return None;
                }
                if (node.right().is_some() && !self.is_red(node.right().unwrap())) && 
                    (node.right().unwrap().left().is_some() && !self.is_red(node.right().unwrap().left().unwrap())) {

                    node = self.move_red_right(node);
                }
                if key == node.key() {
                    let x = self._min_node(node.right().unwrap()).unwrap();
                    node.set_key(x.key());
                    node.set_value(x.value());
                    node.set_right(self._delete_min_node(node.right().unwrap()));
                } else {
                    if node.right().is_some() {
                        let dresult = self.delete_node(node.right().unwrap(), key);
                        
                        if dresult.is_some() {
                            node.set_right(Some(dresult.unwrap()));
                        } else {
                            node.set_right(None);
                        } 
                    } else {
                        return None;
                    }
                }
            }
            Some(self.balance(node))
        }

        // no unsafe
        fn move_red_left(&self, mut node: &'n T) -> &'n T {
            self.flip_colors(node);

            if node.right().is_some() && node.right().unwrap().left().is_some() {
                if self.is_red(node.right().unwrap().left().unwrap()) {
                    node.set_right(Some(self.rotate_right(node.right().unwrap())));
                    node = self.rotate_left(node);
                    self.flip_colors(node);
                }
            }
            node
        }

        // no unsafe
        fn move_red_right(&self, mut node: &'n T) -> &'n T {
            self.flip_colors(node);
            
            if node.left().is_some() && node.left().unwrap().left().is_some() {
                if self.is_red(node.left().unwrap().left().unwrap()) {
                    node = self.rotate_right(node);
                    self.flip_colors(node);
                }
            }
            node
        }

        // no unsafe
        fn balance(&self, mut node: &'n T) -> &'n T {

            if node.right().is_some() && self.is_red(node.right().unwrap()) {
                node = self.rotate_left(node);
            }
            if (node.left().is_some() && self.is_red(node.left().unwrap())) &&
                (node.left().unwrap().left().is_some() && self.is_red(node.left().unwrap().left().unwrap())) {
                node = self.rotate_right(node);
            }
            if (node.left().is_some() && self.is_red(node.left().unwrap())) && 
                (node.right().is_some() && self.is_red(node.right().unwrap())) {

                self.flip_colors(node);
            }
            node.set_n(
                1 
                + if node.left().is_some() { self.node_n(node.left().unwrap()) } else { ZERO_U128 }
                + if node.right().is_some() { self.node_n(node.right().unwrap()) } else { ZERO_U128 },
            );

            node
        }

        // no unsafe
        pub fn print(&self) {
            if self.root().is_none() {
                return;
            }
            self.print_node(self.root().unwrap());
        }

        // no unsafe
        pub fn print_node(&self, node: &T) {
            if node.left().is_some() { self.print_node(node.left().unwrap()); }
            serial_println!("{} {}", node.key(), node.value() as usize);
            if node.right().is_some() { self.print_node(node.right().unwrap()); }
        }

        // no unsafe
        pub fn print_tree(&self) {
            if self.root().is_none() {
                return;
            }
            
            self.print_tree_node(self.root().unwrap(), 0);
        }

        // no unsafe
        pub fn print_tree_node(&self, node: &T, indent: usize) {
            if node.right().is_some() { self.print_tree_node(node.right().unwrap(), indent + 1); }
            
            for _ in 0..indent {
                serial_print!("  ");
            }
            serial_println!("{} {}", node.key(), node.value() as usize);

            if node.left().is_some() { self.print_tree_node(node.left().unwrap(), indent + 1); }
        }

        // no unsafe
        pub fn print_tree_size(&self) {
            if self.root().is_none() {
                return;
            }

            self.print_tree_size_node(self.root().unwrap(), 0);
        }

        pub fn print_tree_size_node(&self, node: &T, indent: usize) {
            if node.right().is_some() { self.print_tree_size_node(node.right().unwrap(), indent + 1); }
            
            for _ in 0..indent {
                serial_print!("  ");
            }            
            serial_println!("{} {}", node.key(), node.value() as usize);
            
            if node.left().is_some() { self.print_tree_size_node(node.left().unwrap(), indent + 1); }
        }
    }
}