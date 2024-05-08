use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// We should not acquire efs lock here.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// create 方法可以在根目录下创建一个文件，
    /// 该方法只有根目录的 Inode 会调用：
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &mut DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.modify_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    /// ls 方法可以收集根目录下的所有文件的文件名并以
    /// 向量的形式返回，这个方法只有根目录的 Inode 才会调用：
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    /// 从根目录索引到一个文件之后可以对它进行读写，
    /// 注意，和 DiskInode 一样，这里的读写作用在字节序列的一段区间上：
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    /// 需要注意在 DiskInode::write_at 之前先调用 increase_size 对自身进行扩容：
    /// 这里会从 EasyFileSystem 中分配一些用于扩容的数据块
    /// 并传给 DiskInode::increase_size 。
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    /// 在以某些标志位打开文件（例如带有 CREATE 标志打开一个已经存在的文件）的时候，
    /// 需要首先将文件清空。在索引到文件的 Inode 之后可以调用 clear 方法：
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }

    /// 硬链接文件
    pub fn link(&self, name: &str, old_name: &str)  {
        let mut fs = self.fs.lock();
        let inode_id = self.read_disk_inode(|disk_inode| {
            self.find_inode_id(old_name, disk_inode)});
        if inode_id.is_none() {
            return;
        }
        let inode_id = inode_id.unwrap();
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            self.increase_size(new_size as u32, root_inode, &mut fs);
            
            let dirent = DirEntry::new(name, inode_id);
            
            root_inode.write_at(
                file_count * DIRENT_SZ,
                 dirent.as_bytes(), 
                 &self.block_device
            );
        });
        block_cache_sync_all();
    }

    /// 通过inode_id查找 direntry
    fn find_by_inode_id(&self, inode_id: u32, disk_inode: &DiskInode) -> i32 {
        assert!(disk_inode.is_dir());
        let mut count = 0;
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.inode_id() == inode_id {
                count += 1;
            }
        }
        count
    }

    /// 取消硬链接文件
    pub fn unlink(&self, name: &str) {
        // let fs = self.fs.lock();
        // let mut inode_id = self.read_disk_inode(|disk_inode| {
        //     self.find_inode_id(name, disk_inode)}).unwrap();
        let mut inode_id = 0;
        let inode = self.find(name).unwrap();
        
        self.modify_disk_inode(|root_inode| {
            // delete file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
           
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    root_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );
                let new_dirent = DirEntry::empty();
                if dirent.name() == name {
                    inode_id = dirent.inode_id();
                    root_inode.write_at(
                        DIRENT_SZ * i,
                        new_dirent.as_bytes(),
                        &self.block_device);
                    break;
                }
            }
            
        });
        let count = self.hard_link_count(inode_id);
        if count == 1{
            inode.clear();
        }
        // block_cache_sync_all();
    }

    /// 通过inode 获得inode_id(ROOT调用)
    pub fn get_inode_id(&self, inode: &Inode) -> u32 {
        let fs = self.fs.lock();
        
        let block_id = inode.block_id;
        let block_offset = inode.block_offset;
        fs.get_inode_id(block_id, block_offset)
    }

    /// inode 是否为 directory（ROOT调用）
    pub fn is_directory(&self, inode: &Inode) -> bool{
        self.block_id == inode.block_id
    }

    /// inode 有几个硬链接
    pub fn hard_link_count(&self, inode_id: u32) -> u32 {
        let mut count = 0;
        self.modify_disk_inode(|root_inode| {
            count = self.find_by_inode_id(inode_id, root_inode);
        });
        count as u32
    }
}
