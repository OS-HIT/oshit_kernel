// struct FAT32CommonFile {...}
// impl CommonFile for FAT32CommonFile {...}

// struct FAT32DirFile {...}
// impl CommonFile for FAT32DirFile {...}

// I would recommand hold an Arc of all opened file to prevent potential race.
