use core::ptr::NonNull;

use super::{page_align_up, page_align_down, PAGE_SIZE};

#[derive(Debug, Clone, Copy)]
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
    fn node(&self) -> Option<&PageNode>;
    fn node_mut(&mut self) -> Option<&mut PageNode>;
    fn parent(&self) -> Option<NonNull<PageNode>>;
    fn left(&self) -> Option<NonNull<PageNode>>;
    fn right(&self) -> Option<NonNull<PageNode>>;
}

impl IsPageLink for PageLink {
    fn node(&self) -> Option<&PageNode> {
        self.map(|n| unsafe { n.as_ref() })
    }

    fn node_mut(&mut self) -> Option<&mut PageNode> {
        self.map(|mut n| unsafe { n.as_mut() })
    }

    fn parent(&self) -> Option<NonNull<PageNode>> {
        self.node().and_then(|n| n.parent)
    }

    fn left(&self) -> Option<NonNull<PageNode>> {
        self.node().and_then(|n| n.left)
    }

    fn right(&self) -> Option<NonNull<PageNode>> {
        self.node().and_then(|n| n.right)
    }
}

struct PageTree {
    root: PageLink,
}

impl PageTree {
    pub fn new() -> PageTree {
        PageTree { root: None }
    }

    // Safety: depends on N, unsafe?
    pub fn insert(&mut self, n: usize) {
        let mut cur = (self.root, None); // (curr, prev)
        while let Some(current) = cur.0.map(|n| unsafe { n.as_ref() }) {
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
        while let Some(Color::Red) = clink.map(|n| n.as_ref().color) {
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
                } else if clink == clink.parent().right() {
                    // TODO:
                }
            }
        }
    }

    pub fn remove(&mut self, npages: usize) -> usize {
        todo!()
    }

    pub fn get_npages(&self) {}
    pub fn get_page(&self) {}

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

}
