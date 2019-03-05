use trie_db::node::Node;
use trie_db::HashDB;
use trie_db::Hasher;
use trie_db::NibbleSlice;
use trie_db::NodeCodec;

pub fn merkle_diff<
    'a,
    'b,
    Codec: NodeCodec<H>,
    T: AsRef<[u8]> + 'a,
    H: Hasher,
    DB: HashDB<H, T>,
>(
    db: &'a DB,
    old_root: &'a <H as Hasher>::Out,
    new_root: &'a <H as Hasher>::Out,
) -> Vec<NodeDiffOwned> {
    let old_root_value = db.get(&old_root).unwrap();
    let old_root_value_node = Codec::decode(old_root_value.as_ref()).unwrap();
    let new_root_value = db.get(&new_root).unwrap();
    let new_root_value_node = Codec::decode(new_root_value.as_ref()).unwrap();

    let diff = diff_nodes(
        NibbleSlice::new(&[]),
        old_root_value_node,
        new_root_value_node,
    );

    vec![diff.into()]
        .into_iter()
        .filter(|n: &NodeDiffOwned| !n.is_empty())
        .collect()
}

#[derive(Debug)]
pub struct NodeDiff<'a, 'b> {
    added_entries: Vec<Entry<'a, 'b>>,
    removed_entries: Vec<Entry<'a, 'b>>,
}

impl<'a, 'b> NodeDiff<'a, 'b> {
    pub fn is_empty(&self) -> bool {
        self.added_entries.is_empty() && self.removed_entries.is_empty()
    }
}

#[derive(Debug, PartialEq)]
pub struct NodeDiffOwned {
    added_entries: Vec<EntryOwned>,
    removed_entries: Vec<EntryOwned>,
}

impl NodeDiffOwned {
    pub fn is_empty(&self) -> bool {
        self.added_entries.is_empty() && self.removed_entries.is_empty()
    }
}

impl<'a, 'b> From<NodeDiff<'a, 'b>> for NodeDiffOwned {
    fn from(from: NodeDiff) -> NodeDiffOwned {
        NodeDiffOwned {
            added_entries: from.added_entries.into_iter().map(|n| n.into()).collect(),
            removed_entries: from.removed_entries.into_iter().map(|n| n.into()).collect(),
        }
    }
}

#[derive(Debug)]
pub struct Entry<'a, 'b> {
    key: NibbleSlice<'a>,
    value: &'b [u8],
}

impl<'a, 'b> Default for NodeDiff<'a, 'b> {
    fn default() -> Self {
        Self {
            added_entries: vec![],
            removed_entries: vec![],
        }
    }
}

impl<'a, 'b> From<Entry<'a, 'b>> for EntryOwned {
    fn from(from: Entry) -> EntryOwned {
        EntryOwned {
            key: from.key.into(),
            value: from.value.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryOwned {
    key: NibbleOwned,
    value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NibbleOwned {
    pub inner: Vec<u8>,
}

impl<'a> From<NibbleSlice<'a>> for NibbleOwned {
    fn from(from: NibbleSlice) -> NibbleOwned {
        NibbleOwned {
            inner: from.iter().collect(),
        }
    }
}

pub fn diff_nodes<'a, 'b>(
    nibble: NibbleSlice<'a>,
    old_node: Node<'b>,
    new_node: Node<'b>,
) -> NodeDiff<'a, 'b> {
    let mut diff = NodeDiff::default();
    if old_node == new_node {
        return diff;
    }

    match (old_node, new_node) {
        (Node::Branch(_, None), Node::Branch(_, Some(value))) => {
            diff.added_entries.push(Entry { key: nibble, value });
        }
        _ => {}
    }

    diff
}

#[cfg(test)]
mod tests {
    use memory_db::*;
    use reference_trie::ReferenceNodeCodec;
    use reference_trie::{RefTrieDBMut, TrieMut};
    use rustc_hex::FromHex;
    use rustc_hex::ToHex;
    use trie_db::NodeCodec;

    fn bytes_hex(input: &str) -> [u8; 32] {
        let parsed: Vec<u8> = input.from_hex().unwrap();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&parsed);

        arr
    }

    #[test]
    fn same_root_has_no_diff() {
        let h1 = bytes_hex("b3179194677268c88cfd1644c6a1e100729465b42846a2bf7f0bddcd07e300a9");
        let h2 = bytes_hex("73df9fe9531a29afa7435bb4564336d0613c2f5ca550dabd9427d8d854e01de5");
        let h3 = bytes_hex("e77fddce0bc5ecd30e3959d43d9dc36ef5448a113b7524621bac9053c02b3319");

        let mut memdb = MemoryDB::default();
        let mut root = Default::default();
        {
            let mut tree = RefTrieDBMut::new(&mut memdb, &mut root);

            tree.insert(&h1, b"bar").unwrap();
            tree.insert(&h2, b"foo").unwrap();
            tree.insert(&h3, b"baz").unwrap();
        }

        assert_eq!(
            root,
            bytes_hex("7bbd6c88f3e499e909c2ad4a589b35bdce6ab91d3b436a428447fa30ec25e20d")
        );
        let diff = crate::merkle_diff::<ReferenceNodeCodec, _, _, _>(&memdb, &root, &root);
        assert!(diff.is_empty());
    }

    #[test]
    fn branch_immediate_added() {
        let mut memdb = MemoryDB::default();
        let mut old_root = Default::default();
        {
            let mut tree = RefTrieDBMut::new(&mut memdb, &mut old_root);
            tree.insert(&[0u8], b"bar").unwrap();
            tree.insert(&[20u8], b"bar").unwrap();
        }
        let mut new_root = Default::default();
        {
            let mut tree = RefTrieDBMut::new(&mut memdb, &mut new_root);
            tree.insert(&[0u8], b"bar").unwrap();
            tree.insert(&[20u8], b"bar").unwrap();
            tree.insert(&[], b"baz").unwrap();
        }
        {
            // let query = |n: &[u8]| n.to_owned();
            // let lookup = trie_db::Lookup::<_, ReferenceNodeCodec, _> {
            // db: &memdb,
            // query,
            // hash: new_root,
            // marker: std::marker::PhantomData,
            // };
            // let res = lookup.look_up(trie_db::NibbleSlice::new(&[]));
        }

        let diff = crate::merkle_diff::<ReferenceNodeCodec, _, _, _>(&memdb, &old_root, &new_root);
        assert!(!diff.is_empty());
        assert_eq!(1, diff[0].added_entries.len());
        assert_eq!(
            crate::EntryOwned {
                key: crate::NibbleOwned { inner: vec![] },
                value: b"baz".to_vec()
            },
            diff[0].added_entries[0]
        );
    }

    #[test]
    #[ignore]
    fn it_works() {
        let h1 = bytes_hex("b3179194677268c88cfd1644c6a1e100729465b42846a2bf7f0bddcd07e300a9");
        let h2 = bytes_hex("73df9fe9531a29afa7435bb4564336d0613c2f5ca550dabd9427d8d854e01de5");
        let h3 = bytes_hex("e77fddce0bc5ecd30e3959d43d9dc36ef5448a113b7524621bac9053c02b3319");

        let mut memdb = MemoryDB::default();
        let old_root = {
            let mut root2 = Default::default();
            let mut tree2 = RefTrieDBMut::new(&mut memdb, &mut root2);

            tree2.insert(&h1, b"bar").unwrap();
            tree2.insert(&h2, b"foo").unwrap();
            // tree.insert(&h2, b"baz").unwrap();
            let root2 = tree2.root().to_owned();
            println!("Root: {}", root2.to_hex::<String>());
            let root_value2 = tree2.db().get(&root2).unwrap();
            let root_value_node2 = ReferenceNodeCodec::decode(&root_value2).unwrap();
            println!("Valu: {:?}", root_value_node2);

            root2
        };
        let new_root = {
            let mut root = Default::default();
            let mut tree = RefTrieDBMut::new(&mut memdb, &mut root);

            tree.insert(&h1, b"bar").unwrap();
            tree.insert(&h2, b"foo").unwrap();
            tree.insert(&h3, b"baz").unwrap();
            let root = tree.root().to_owned();
            println!("Root: {}", root.to_hex::<String>());
            let root_value = tree.db().get(&root).unwrap();
            let root_value_node = ReferenceNodeCodec::decode(&root_value).unwrap();
            println!("Valu: {:?}", root_value_node);

            root
        };

        let diff = crate::merkle_diff::<ReferenceNodeCodec, _, _, _>(&memdb, &old_root, &new_root);
        println!("DIFF: {:?}", diff);
    }
}
