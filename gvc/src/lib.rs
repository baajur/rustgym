use gvc_sys::*;
use rustgym_util::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ffi::CString;

type Attrs = Vec<(CString, CString, CString)>;

fn make_attrs(pairs: Vec<(&str, &str)>) -> Attrs {
    pairs
        .into_iter()
        .map(|p| {
            (
                CString::new(p.0.to_string()).expect("key"),
                CString::new(p.1.to_string()).expect("value"),
                CString::new("").expect("default"),
            )
        })
        .collect()
}

fn set_attrs(obj: *mut c_void, attrs: Attrs) {
    unsafe {
        for (k, v, d) in attrs {
            agsafeset(obj as *mut c_void, k.into_raw(), v.into_raw(), d.into_raw());
        }
    }
}

pub struct GSVG {
    gvc: *mut GVC_t,
    graph: *mut Agraph_t,
    nodes: Vec<*mut Agnode_t>,
    edges: Vec<*mut Agedge_t>,
}

impl GSVG {
    pub fn new(name: &str) -> Self {
        let g_attrs = make_attrs(vec![("label", name), ("rankdir", "TB")]);
        unsafe {
            let graph = agopen(
                g_attrs[0].clone().1.into_raw(),
                Agundirected,
                std::ptr::null_mut::<Agdisc_s>(),
            );
            set_attrs(graph as *mut c_void, g_attrs);
            GSVG {
                gvc: gvContext(),
                graph,
                nodes: vec![],
                edges: vec![],
            }
        }
    }

    pub fn node(&mut self, val: i32) {
        let n_attrs = make_attrs(vec![
            ("id", &self.nodes.len().to_string()),
            ("label", &val.to_string()),
            ("shape", "circle"),
            ("style", "filled"),
        ]);
        unsafe {
            let node = agnode(self.graph, n_attrs[0].clone().1.into_raw(), 1);
            set_attrs(node as *mut c_void, n_attrs);
            self.nodes.push(node);
        }
    }

    pub fn edge(&mut self, t: usize, h: usize, forward: bool, dotted: bool) {
        let e_attrs = make_attrs(vec![
            ("id", &self.edges.len().to_string()),
            ("dir", if forward { "forward" } else { "none" }),
            ("style", if dotted { "dotted" } else { "filled" }),
        ]);
        unsafe {
            let e = agedge(
                self.graph,
                self.nodes[t],
                self.nodes[h],
                e_attrs[0].clone().1.into_raw(),
                1,
            );
            set_attrs(e as *mut c_void, e_attrs);
            self.edges.push(e);
        }
    }

    pub fn update_nodes(&self, n_attrs: Vec<HashMap<String, String>>) {
        assert_eq!(self.nodes.len(), n_attrs.len());
        for (i, attrs) in n_attrs.into_iter().enumerate() {
            for (key, value) in attrs {
                let key: CString = CString::new(key).expect("key");
                let value: CString = CString::new(value).expect("value");
                unsafe {
                    agsafeset(
                        self.nodes[i] as *mut c_void,
                        key.into_raw(),
                        value.into_raw(),
                        CString::new("").expect("default").into_raw(),
                    );
                }
            }
        }
    }

    pub fn render(&self, filename: &str) {
        unsafe {
            gvLayout(
                self.gvc,
                self.graph,
                CString::new("dot").expect("dot").into_raw(),
            );
            gvRenderFilename(
                self.gvc,
                self.graph,
                CString::new("svg").expect("svg").into_raw(),
                CString::new(filename).expect("svg").into_raw(),
            );
            gvFreeLayout(self.gvc, self.graph); /* ignore errors */
        }
    }
}

impl Drop for GSVG {
    fn drop(&mut self) {
        unsafe {
            agclose(self.graph);
            gvFreeContext(self.gvc);
        }
    }
}

pub trait Draw {
    fn draw(&self, caption: &str) -> GSVG;
}

pub trait DrawTree {
    fn draw_r(&self, parent_id: Option<usize>, id: &mut usize, gsvg: &mut GSVG);
}

impl DrawTree for TreeLink {
    fn draw_r(&self, parent_id: Option<usize>, id: &mut usize, gsvg: &mut GSVG) {
        if let Some(node) = self {
            let node = node.borrow();
            let nid = *id;
            let val = node.val;
            gsvg.node(val);
            if let Some(pid) = parent_id {
                gsvg.edge(pid, nid, false, false);
            }
            *id += 1;
            let left = &node.left;
            let right = &node.right;
            left.draw_r(Some(nid), id, gsvg);
            right.draw_r(Some(nid), id, gsvg);
        }
    }
}

impl Draw for TreeLink {
    fn draw(&self, caption: &str) -> GSVG {
        let mut gsvg = GSVG::new(caption);
        let mut id = 0;
        self.draw_r(None, &mut id, &mut gsvg);
        gsvg
    }
}

impl Draw for Vec<Vec<usize>> {
    fn draw(&self, caption: &str) -> GSVG {
        let mut gsvg = GSVG::new(caption);
        let n = self.len();
        for i in 0..n {
            gsvg.node(i as i32);
        }
        for (u, vs) in self.iter().enumerate() {
            for &v in vs {
                gsvg.edge(u, v, false, false);
            }
        }
        gsvg
    }
}

impl Draw for Graph {
    fn draw(&self, caption: &str) -> GSVG {
        let mut gsvg = GSVG::new(caption);
        let n = self.n;
        for i in 0..n {
            gsvg.node(i as i32);
        }
        for e in self.edges.iter() {
            match e.kind {
                EdgeKind::TreeEdge => gsvg.edge(e.tail, e.head, true, false),
                _ => gsvg.edge(e.tail, e.head, true, true),
            }
        }
        gsvg
    }
}

// #[test]
// fn test_tree() {
//     let root: TreeLink = tree!(1, tree!(2, tree!(3), None), tree!(4));
//     let gsvg = root.draw("Test Tree");
//     gsvg.render("test_tree.svg");
// }

// #[test]
// fn test_graph() {
//     let n = 5;
//     let mut graph: Graph = Graph::new(n);
//     let edges = vec_vec_i32![[1, 0], [2, 0], [3, 0], [4, 1], [4, 2], [4, 0]];
//     graph.init_with_edges(edges, false);
//     graph.travase();
//     let gsvg = graph.draw("Test Graph");
//     gsvg.render("test_graph.svg");
// }
