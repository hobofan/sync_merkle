use trie_db::NibbleSlice;

#[derive(Debug)]
pub struct NodeDiff<'a, 'b> {
    pub added_entries: Vec<Entry<'a, 'b>>,
    pub removed_entries: Vec<Entry<'a, 'b>>,
}

impl<'a, 'b> NodeDiff<'a, 'b> {
    pub fn is_empty(&self) -> bool {
        self.added_entries.is_empty() && self.removed_entries.is_empty()
    }
}

#[derive(Debug, PartialEq)]
pub struct NodeDiffOwned {
    pub added_entries: Vec<EntryOwned>,
    pub removed_entries: Vec<EntryOwned>,
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
    pub key: NibbleSlice<'a>,
    pub value: &'b [u8],
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
    pub key: NibbleOwned,
    pub value: Vec<u8>,
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
