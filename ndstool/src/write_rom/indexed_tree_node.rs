use crate::source::SourceTreeNode;

pub struct IndexedTreeNode {
    pub name: String,
    pub children: Option<Vec<IndexedTreeNode>>,
    pub dir_id: u16,
    pub dir_count: u16,
}

impl IndexedTreeNode {
    fn is_dir(&self) -> bool {
        self.children.is_some()
    }

    pub fn from_source_tree_node(node: &SourceTreeNode, free_dir_id: &mut u16) -> IndexedTreeNode {
        let name = node.name.clone();
        match &node.children {
            Some(child_nodes) => {
                let dir_id = *free_dir_id;
                *free_dir_id += 1;
                let indexed_child_nodes: Vec<IndexedTreeNode> = child_nodes
                    .iter()
                    .map(|node| Self::from_source_tree_node(node, free_dir_id))
                    .collect();
                let dir_count = 1 + indexed_child_nodes
                    .iter()
                    .map(|node| node.dir_count)
                    .sum::<u16>();
                IndexedTreeNode {
                    name,
                    children: Some(indexed_child_nodes),
                    dir_id,
                    dir_count,
                }
            }
            None => IndexedTreeNode {
                name,
                children: None,
                dir_id: 0,
                dir_count: 0,
            },
        }
    }

    fn name_bytes(&self) -> Vec<u8> {
        let first_byte: u8 =
            ((self.name.len() as u8) & 0x7F) | (if self.children.is_some() { 0x80 } else { 0x00 });
        let mut name_bytes: Vec<u8> = std::iter::once(first_byte)
            .chain(self.name.bytes())
            .collect();
        if self.children.is_some() {
            name_bytes.extend_from_slice(&(self.dir_id as u16).to_le_bytes());
        }
        name_bytes
    }

    fn dir_name_table(
        &self,
        parent_id: u16,
        free_name_bytes_pos: &mut u32,
        free_file_id: &mut u16,
    ) -> Option<(Vec<(u32, u16, u16)>, Vec<u8>)> {
        let child_nodes = match &self.children {
            Some(child_nodes) => child_nodes,
            None => return None,
        };
        let mut dir_data = vec![(*free_name_bytes_pos, *free_file_id, parent_id)];
        // Iterate through files that are direct children first
        let child_files_name_data = child_nodes
            .iter()
            .filter(|node| !node.is_dir())
            .flat_map(|node| node.name_bytes());
        // Then iterate through subdirectories
        let child_dirs_name_data = child_nodes
            .iter()
            .filter(|node| node.is_dir())
            .flat_map(|node| node.name_bytes());
        let mut name_data: Vec<u8> = child_files_name_data.chain(child_dirs_name_data).collect();
        name_data.push(0);
        *free_name_bytes_pos += name_data.len() as u32;
        *free_file_id += child_nodes.iter().filter(|node| !node.is_dir()).count() as u16;
        for child_node in child_nodes {
            if let Some((child_dir_data, child_name_data)) =
                child_node.dir_name_table(self.dir_id, free_name_bytes_pos, free_file_id)
            {
                dir_data.extend(&child_dir_data);
                name_data.extend(&child_name_data);
            }
        }
        Some((dir_data, name_data))
    }

    pub fn name_table(&self, free_file_id: &mut u16) -> (Vec<(u32, u16, u16)>, Vec<u8>) {
        let mut free_name_bytes_pos: u32 = 8 * (self.dir_count as u32);
        self.dir_name_table(self.dir_count, &mut free_name_bytes_pos, free_file_id)
            .unwrap()
    }
}
