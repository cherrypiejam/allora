use core::ptr::NonNull;

use super::{page_align_up, page_align_down, PAGE_SIZE};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Color {
    Red,
    Black,
}

type PageLink = Option<NonNull<PageNode>>;

struct PageNode {
    n: usize,
    parent: PageLink,
    left: PageLink,
    right: PageLink,
    color: Color,
}

trait IsPageLink {
    unsafe fn node(&self) -> Option<&PageNode>;
    unsafe fn node_mut(&mut self) -> Option<&mut PageNode>;
    unsafe fn parent(&self) -> Option<NonNull<PageNode>>;
    unsafe fn left(&self) -> Option<NonNull<PageNode>>;
    unsafe fn right(&self) -> Option<NonNull<PageNode>>;
}

impl IsPageLink for PageLink {
    unsafe fn node(&self) -> Option<&PageNode> {
        self.map(|n| n.as_ref())
    }

    unsafe fn node_mut(&mut self) -> Option<&mut PageNode> {
        self.map(|mut n| n.as_mut())
    }

    unsafe fn parent(&self) -> Option<NonNull<PageNode>> {
        self.node().and_then(|n| n.parent)
    }

    unsafe fn left(&self) -> Option<NonNull<PageNode>> {
        self.node().and_then(|n| n.left)
    }

    unsafe fn right(&self) -> Option<NonNull<PageNode>> {
        self.node().and_then(|n| n.right)
    }
}

pub struct PageTree {
    root: PageLink,
}

unsafe impl Send for PageTree {}

impl PageTree {
    pub fn empty() -> PageTree {
        PageTree { root: None }
    }

    pub unsafe fn new(start: usize, size: usize) -> PageTree {
        let mut pt = Self::empty();
        pt.init(start, size);
        pt
    }

    pub unsafe fn init(&mut self, start: usize, size: usize) {
        let base = page_align_up(start) / PAGE_SIZE;
        let npages = page_align_down(size) / PAGE_SIZE;

        (base..base+npages)
            .for_each(|n| self.insert(n))
    }

    // Safety: depends on the given page number
    pub unsafe fn insert(&mut self, n: usize) {
        let mut cur = (self.root, None); // (curr, prev)
        while let Some(current) = cur.0.map(|n| n.as_ref()) {
            cur.1 = cur.0;
            if n < current.n {
                cur.0 = current.left;
            } else {
                cur.0 = current.right;
            }
        }
        let parent = cur.1;
        let ptr = unsafe {
            let ptr = (PAGE_SIZE * n) as *mut PageNode;
            ptr.write(PageNode {
                n, parent, left: None, right: None, color: Color::Red,
            });
            Some(NonNull::new_unchecked(ptr))
        };
        if let Some(p) = parent.map(|mut n| unsafe { n.as_mut() }) {
            if n < p.n {
                p.left = ptr;
            } else {
                p.right = ptr;
            }
        } else {
            self.root = ptr;
        }
        unsafe { self.insert_fixup(ptr) };
    }

    unsafe fn insert_fixup(&mut self, mut clink: Option<NonNull<PageNode>>) {
        while let Some(Color::Red) = clink.parent().node().map(|n| n.color) {
            if clink.parent() == clink.parent().parent().left() {
                let mut blink = clink.parent().parent().right();
                if let Some(Color::Red) = blink.map(|n| n.as_ref().color) {
                    if let Some(n) = clink.parent().node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = blink.node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = clink.parent().parent().node_mut() {
                        n.color = Color::Red;
                    }
                    clink = clink.parent().parent()
                } else {
                    if clink == clink.parent().right() {
                        clink = clink.parent();
                        self.left_rotate(clink.node_mut().unwrap());
                    }
                    if let Some(n) = clink.parent().node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = clink.parent().parent().node_mut() {
                        n.color = Color::Red;
                    }
                    if let Some(n) = clink.parent().parent().node_mut() {
                        self.right_rotate(n);
                    }
                }
            } else {
                let mut blink = clink.parent().parent().left();
                if let Some(Color::Red) = blink.map(|n| n.as_ref().color) {
                    if let Some(n) = clink.parent().node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = blink.node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = clink.parent().parent().node_mut() {
                        n.color = Color::Red;
                    }
                    clink = clink.parent().parent()
                } else {
                    if clink == clink.parent().left() {
                        clink = clink.parent();
                        self.right_rotate(clink.node_mut().unwrap());
                    }
                    if let Some(n) = clink.parent().node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = clink.parent().parent().node_mut() {
                        n.color = Color::Red;
                    }
                    self.left_rotate(clink.parent().parent().node_mut().unwrap())
                }
            }
        }
        self.root.node_mut().unwrap().color = Color::Black;
    }

    pub fn get(&mut self) -> Option<usize> {
        self.get_multiple(1)
    }

    pub fn get_multiple(&mut self, npages: usize) -> Option<usize> {
        let found = unsafe {
            let mut cur = Self::min_link(self.root);
            while cur.is_some() && !Self::is_n_consecutive_link(cur, npages) {
                cur = Self::next_link(cur);
            }
            cur.node().map(|n| n.n)
        };

        if let Some(start) = found {
            (start..(start+npages))
                .for_each(|n| unsafe { self.delete(n) })
        }

        found
    }

    // Safety: depends on the given page number
    unsafe fn delete(&mut self, n: usize) {
        let clink = NonNull::new((PAGE_SIZE * n) as *mut PageNode);
        let mut blink = clink;
        let mut alink: PageLink;
        let mut bcolor = blink.node().map(|b| b.color).unwrap_or(Color::Red);
        if clink.left().is_none() {
            alink = clink.right();
            self.transplant(clink, clink.right());
        } else if clink.right().is_none() {
            alink = clink.left();
            self.transplant(clink, clink.left());
        } else {
            blink = Self::min_link(clink.right());
            if let Some(c) = blink.node().map(|n| n.color) {
                bcolor = c;
            }
            alink = blink.right();
            if blink != clink.right() {
                self.transplant(blink, blink.right());
                if let Some(n) = blink.node_mut() {
                    n.right = clink.right();
                }
                if let Some(n) = blink.right().node_mut() {
                    n.parent = blink;
                }
            } else {
                if let Some(n) = alink.node_mut() {
                    n.parent = blink;
                }
            }
            self.transplant(clink, blink);
            if let Some(n) = blink.node_mut() {
                n.left = clink.left();
                n.color = clink.node().unwrap().color;
            }
            if let Some(n) = blink.left().node_mut() {
                n.parent = blink;
            }
        }
        if bcolor == Color::Black {
            unsafe { self.delete_fixup(alink) }
        }
    }

    unsafe fn delete_fixup(&mut self, mut alink: Option<NonNull<PageNode>>) {
        while alink != self.root && alink.node().map(|n| n.color) == Some(Color::Black) {
            if alink == alink.parent().left() {
                let mut blink = alink.parent().right();
                if let Some(Color::Red) = blink.node().map(|n| n.color) {
                    if let Some(n) = blink.node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = alink.parent().node_mut() {
                        n.color = Color::Red;
                    }
                    self.left_rotate(alink.parent().node_mut().unwrap());
                    blink = alink.parent().right();
                }
                if blink.left().node().unwrap().color == Color::Black
                   && blink.right().node().unwrap().color == Color::Black
                {
                    if let Some(n) = blink.node_mut() {
                        n.color = Color::Red;
                    }
                    alink = alink.parent();
                } else {
                    if blink.right().node().unwrap().color == Color::Black {
                        if let Some(n) = blink.left().node_mut() {
                            n.color = Color::Black;
                        }
                        if let Some(n) = blink.node_mut() {
                            n.color = Color::Red;
                        }
                        self.right_rotate(blink.node_mut().unwrap());
                        blink = alink.parent().right();
                    }
                    if let Some(n) = blink.node_mut() {
                        n.color = alink.parent().node().unwrap().color;
                    }
                    if let Some(n) = alink.parent().node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = blink.right().node_mut() {
                        n.color = Color::Black;
                    }
                    self.left_rotate(alink.parent().node_mut().unwrap());
                    alink = self.root;
                }
            } else {
                let mut blink = alink.parent().left();
                if let Some(Color::Red) = blink.node().map(|n| n.color) {
                    if let Some(n) = blink.node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = alink.parent().node_mut() {
                        n.color = Color::Red;
                    }
                    self.right_rotate(alink.parent().node_mut().unwrap());
                    blink = alink.parent().left();
                }
                if blink.right().node().unwrap().color == Color::Black
                   && blink.left().node().unwrap().color == Color::Black
                {
                    if let Some(n) = blink.node_mut() {
                        n.color = Color::Red;
                    }
                    alink = alink.parent();
                } else {
                    if blink.left().node().unwrap().color == Color::Black {
                        if let Some(n) = blink.right().node_mut() {
                            n.color = Color::Black;
                        }
                        if let Some(n) = blink.node_mut() {
                            n.color = Color::Red;
                        }
                        self.left_rotate(blink.node_mut().unwrap());
                        blink = alink.parent().left();
                    }
                    if let Some(n) = blink.node_mut() {
                        n.color = alink.parent().node().unwrap().color;
                    }
                    if let Some(n) = alink.parent().node_mut() {
                        n.color = Color::Black;
                    }
                    if let Some(n) = blink.left().node_mut() {
                        n.color = Color::Black;
                    }
                    self.right_rotate(alink.parent().node_mut().unwrap());
                    alink = self.root;
                }
            }
        }
        if let Some(n) = alink.node_mut() {
            n.color = Color::Black;
        }
    }

    // FIXME: failed after exceeding limits, support custom allocator or impl Iterator instead
    // Safety: calling it may corrupt the stack
    pub unsafe fn traverse(&mut self) -> heapless::Vec<usize, 1024> {
        let mut stack = heapless::Vec::<PageLink, 1024>::new();
        let mut items = heapless::Vec::<usize, 1024>::new();
        let mut visited = heapless::LinearMap::<PageLink, (), 1024>::new();
        if self.root.is_some() {
            stack.push(self.root).unwrap();
        }
        unsafe {
            while let Some(link) = stack.pop() {
                if link.left().is_some() && !visited.contains_key(&link.left()) {
                    stack.push(link).unwrap();
                    stack.push(link.left()).unwrap();
                } else if link.left().is_none() || visited.contains_key(&link.left()) {
                    items.push(link.node().unwrap().n).unwrap();
                    visited.insert(link, ()).unwrap();
                    if link.right().is_some() {
                        stack.push(link.right()).unwrap();
                    }
                }
            }
        }
        items
    }

    fn left_rotate(&mut self, a: &mut PageNode) {
        let b_ptr = a.right;
        let a_ptr = NonNull::new(a);
        let b = unsafe { b_ptr.unwrap().as_mut() };
        a.right = b.left;
        if let Some(bleft) = b.left.map(|mut n| unsafe { n.as_mut() }) {
            bleft.parent = a_ptr;
        }
        b.parent = a.parent;
        if let Some(aparent) = a.parent.map(|mut n| unsafe { n.as_mut() }) {
            if a_ptr == aparent.left {
                aparent.left = b_ptr;
            } else {
                aparent.right = b_ptr;
            }
        } else {
            self.root = b_ptr;
        }
        b.left = a_ptr;
        a.parent = b_ptr;
    }

    fn right_rotate(&mut self, b: &mut PageNode) {
        let a_ptr = b.left;
        let b_ptr = NonNull::new(b);
        let a = unsafe { a_ptr.unwrap().as_mut() };
        b.left = a.right;
        if let Some(aright) = a.right.map(|mut n| unsafe { n.as_mut() }) {
            aright.parent = b_ptr;
        }
        a.parent = b.parent;
        if let Some(bparent) = b.parent.map(|mut n| unsafe { n.as_mut() }) {
            if b_ptr == bparent.left {
                bparent.left = a_ptr;
            } else {
                bparent.right = a_ptr;
            }
        } else {
            self.root = a_ptr;
        }
        a.right = b_ptr;
        b.parent = a_ptr;
    }

    unsafe fn transplant(&mut self, alink: PageLink, mut blink: PageLink) {
        if let Some(_) = alink.parent() {
            if alink == alink.parent().left() {
                if let Some(n) = alink.parent().node_mut() {
                    n.left = blink;
                }
            } else {
                if let Some(n) = alink.parent().node_mut() {
                    n.right = blink;
                }
            }
        } else {
            self.root = blink;
        }
        if let Some(n) = blink.node_mut() {
            n.parent = alink.parent();
        }
    }

    unsafe fn min_link(start: PageLink) -> PageLink {
        let mut cur = start;
        while cur.left().is_some() {
            cur = cur.left()
        }
        cur
    }

    unsafe fn max_link(start: PageLink) -> PageLink {
        let mut cur = start;
        while cur.right().is_some() {
            cur = cur.right()
        }
        cur
    }

    unsafe fn next_link(start: PageLink) -> PageLink {
        if start.right().is_some() {
            Self::min_link(start.right())
        } else {
            let mut cur = start;
            while cur == cur.parent().right() {
                cur = cur.parent();
            }
            cur.parent()
        }
    }

    unsafe fn is_n_consecutive_link(start: PageLink, npages: usize) -> bool {
        let mut cur = start;
        for i in (1..npages) {
            let next = Self::next_link(cur);
            if next.node().map(|n| n.n) == start.node().map(|n| n.n + i) {
                cur = next;
            } else {
                return false
            }
        }
        true
    }

}


#[cfg(test)]
mod test {
    use super::*;
    use crate::HEAP_START;
    const SIZE: usize = 500_000_000;

    #[test_case]
    fn test_page_tree_primitives() {
        let mut pt = PageTree::empty();
        let base = unsafe { &HEAP_START } as *const _ as usize + SIZE;
        let base = page_align_up(base) / PAGE_SIZE;

        unsafe {
            pt.insert(base+2);
            pt.insert(base+1);
            pt.insert(base);
            pt.insert(base+9);
            pt.insert(base+8);
            assert_eq!(&*pt.traverse(), [base, base+1, base+2, base+8, base+9]);

            pt.insert(base+5);
            assert_eq!(&*pt.traverse(), [base, base+1, base+2, base+5, base+8, base+9]);

            pt.delete(base+2);
            assert_eq!(&*pt.traverse(), [base, base+1, base+5, base+8, base+9]);
            pt.delete(base);
            assert_eq!(&*pt.traverse(), [base+1, base+5, base+8, base+9]);
            pt.delete(base+9);
            assert_eq!(&*pt.traverse(), [base+1, base+5, base+8]);
            pt.delete(base+5);
            assert_eq!(&*pt.traverse(), [base+1, base+8]);
            pt.delete(base+1);
            assert_eq!(&*pt.traverse(), [base+8]);
            pt.delete(base+8);
            assert_eq!(&*pt.traverse(), []);
        }
    }

    #[test_case]
    fn test_page_tree() {
        let mut pt = PageTree::empty();
        let start = unsafe { &HEAP_START } as *const _ as usize + SIZE;
        let base = page_align_up(start) / PAGE_SIZE;

        unsafe {
            pt.init(start, PAGE_SIZE * 3);

            assert_eq!(pt.get_multiple(2), Some(base));
            assert_eq!(&*pt.traverse(), [base+2]);

            assert_eq!(pt.get(), Some(base+2));
            assert_eq!(&*pt.traverse(), []);

            assert_eq!(pt.get(), None);
            assert_eq!(&*pt.traverse(), []);
        }
    }

    #[test_case]
    fn test_page_tree_remove_multiple() {
        let mut pt = PageTree::empty();
        let base = unsafe { &HEAP_START } as *const _ as usize + SIZE;
        let base = page_align_up(base) / PAGE_SIZE;

        unsafe {
            pt.insert(base+2);
            pt.insert(base+1);
            pt.insert(base);
            let start = PageTree::min_link(pt.root);
            assert_eq!(start.node().map(|n| n.n), Some(base));
            let next = PageTree::next_link(start);
            assert_eq!(next.node().map(|n| n.n), Some(base+1));
            let next = PageTree::next_link(next);
            assert_eq!(next.node().map(|n| n.n), Some(base+2));
            let next = PageTree::next_link(next);
            assert_eq!(next.node().map(|n| n.n), None);


            pt.insert(base+9);
            pt.insert(base+8);

            assert_eq!(pt.get_multiple(1), Some(base));
            assert_eq!(pt.get_multiple(2), Some(base+1));
            assert_eq!(pt.get_multiple(3), None);
            assert_eq!(pt.get_multiple(4), None);
            assert_eq!(pt.get_multiple(2), Some(base+8));
        }

    }
}
