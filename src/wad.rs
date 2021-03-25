use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::mem::MaybeUninit;
use std::path::Path;

use byteorder::ReadBytesExt;
use byteorder::LE;

use crate::Error;

#[derive(Debug)]
pub struct Header {
    id: [u8; 4],
    dir_ct: i32,
    dir_ptr: i32,
}

#[derive(Debug)]
struct RawLump {
    ptr: i32,
    size: i32,
    name: [u8; 8],
}

#[derive(Debug)]
pub struct Lump {
    name: String,
    data: Vec<u8>,
    kind: LumpKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LumpKind {
    Things,
    Linedefs,
    Sidedefs,
    Vertexes,
    Segs,
    Subsectors,
    Nodes,
    Sectors,
    Reject,
    Blockmap,
    Behavior,
    Scripts,

    Other,
}

#[derive(Debug)]
pub struct Map {
    name: String,
    things: Lump,
    linedefs: Lump,
    sidedefs: Lump,
    vertexes: Lump,
    segs: Lump,
    subsectors: Lump,
    nodes: Lump,
    sectors: Lump,
    reject: Option<Lump>,
    blockmap: Lump,
    behavior: Option<Lump>,
    scripts: Option<Lump>,
}

const REQUIRED_MAP_COMPONENTS: [LumpKind; 9] = [
    LumpKind::Things,
    LumpKind::Linedefs,
    LumpKind::Sidedefs,
    LumpKind::Vertexes,
    LumpKind::Segs,
    LumpKind::Subsectors,
    LumpKind::Nodes,
    LumpKind::Sectors,
    LumpKind::Blockmap,
];

const OPTIONAL_MAP_COMPONENTS: [LumpKind; 3] =
    [LumpKind::Reject, LumpKind::Behavior, LumpKind::Scripts];

#[derive(Debug)]
pub struct Flat(RawLump);
#[derive(Debug)]
pub struct Sprite(RawLump);

#[derive(Debug)]
pub struct Wad {
    header: Header,
    lumps: Vec<Lump>,
    maps: Vec<Map>,
    sounds: Vec<Lump>,
    music: Vec<Lump>,
    playpal: Option<Lump>,
    colormap: Option<Lump>,
    endoom: Option<Lump>,
    textures: [Option<Lump>; 2],
    demos: [Option<Lump>; 3],
}

impl Wad {
    fn new(header: Header) -> Self {
        Self {
            header,
            lumps: vec![],
            maps: vec![],
            sounds: vec![],
            music: vec![],
            playpal: None,
            colormap: None,
            endoom: None,
            textures: [None, None],
            demos: [None, None, None],
        }
    }

    pub(crate) fn from_file(file: impl AsRef<Path>) -> Result<Self, Error> {
        let mut f = File::open(file.as_ref())?;
        let mut header: Header = unsafe { MaybeUninit::uninit().assume_init() };
        f.read_exact(&mut header.id)?;
        if &header.id != b"IWAD" && &header.id != b"PWAD" {
            return Err(Error::NotAWad(file.as_ref().to_owned()));
        }
        header.dir_ct = f.read_i32::<LE>()?;
        header.dir_ptr = f.read_i32::<LE>()?;

        let mut wad = Wad::new(header);
        f.seek(SeekFrom::Start(wad.header.dir_ptr as u64))?;
        let mut possible_map_name = String::new();
        let mut map_components = HashMap::<LumpKind, Lump>::new();
        for _ in 0..wad.header.dir_ct {
            let mut raw_lump: RawLump = unsafe { MaybeUninit::uninit().assume_init() };
            raw_lump.ptr = f.read_i32::<LE>()?;
            raw_lump.size = f.read_i32::<LE>()?;
            f.read_exact(&mut raw_lump.name)?;

            let lump_name = raw_lump
                .name
                .iter()
                .cloned()
                .take_while(|&b| b != b'\0')
                .collect::<Vec<_>>();

            let mut lump = Lump {
                name: String::from_utf8(lump_name)?.to_uppercase(),
                data: Vec::with_capacity(raw_lump.size as usize),
                kind: LumpKind::Other,
            };

            let old_pos = f.stream_position()?;
            f.seek(SeekFrom::Start(raw_lump.ptr as u64))?;

            lump.data.resize(raw_lump.size as usize, 0u8);
            f.read_exact(&mut lump.data)?;

            f.seek(SeekFrom::Start(old_pos))?;

            lump.kind = match lump.name.as_str() {
                "THINGS" => LumpKind::Things,
                "LINEDEFS" => LumpKind::Linedefs,
                "SIDEDEFS" => LumpKind::Sidedefs,
                "VERTEXES" => LumpKind::Vertexes,
                "SEGS" => LumpKind::Segs,
                "SSECTORS" => LumpKind::Subsectors,
                "NODES" => LumpKind::Nodes,
                "SECTORS" => LumpKind::Sectors,
                "REJECT" => LumpKind::Reject,
                "BLOCKMAP" => LumpKind::Blockmap,
                "BEHAVIOR" => LumpKind::Behavior,
                "SCRIPTS" => LumpKind::Scripts,
                _ => LumpKind::Other,
            };

            if map_components.contains_key(&lump.kind) {
                return Err(Error::InvalidLumpOrder(raw_lump.ptr, lump.name));
            } else if REQUIRED_MAP_COMPONENTS.contains(&lump.kind)
                || OPTIONAL_MAP_COMPONENTS.contains(&lump.kind)
            {
                map_components.insert(lump.kind, lump);
            } else if REQUIRED_MAP_COMPONENTS
                .iter()
                .all(|c| map_components.contains_key(c))
            {
                wad.maps.push(Map {
                    name: std::mem::take(&mut possible_map_name),
                    things: map_components.remove(&LumpKind::Things).unwrap(),
                    linedefs: map_components.remove(&LumpKind::Linedefs).unwrap(),
                    sidedefs: map_components.remove(&LumpKind::Sidedefs).unwrap(),
                    vertexes: map_components.remove(&LumpKind::Vertexes).unwrap(),
                    segs: map_components.remove(&LumpKind::Segs).unwrap(),
                    subsectors: map_components.remove(&LumpKind::Subsectors).unwrap(),
                    nodes: map_components.remove(&LumpKind::Nodes).unwrap(),
                    sectors: map_components.remove(&LumpKind::Sectors).unwrap(),
                    reject: map_components.remove(&LumpKind::Reject),
                    blockmap: map_components.remove(&LumpKind::Blockmap).unwrap(),
                    behavior: map_components.remove(&LumpKind::Behavior),
                    scripts: map_components.remove(&LumpKind::Scripts),
                });
                debug_assert!(map_components.is_empty());
                possible_map_name = lump.name;
            } else {
                if !map_components.is_empty() {
                    return Err(Error::InvalidLumpOrder(raw_lump.ptr, lump.name));
                }
                if lump.data.len() >= 4 {
                    // check for music header
                    let mut header = [0u8; 4];
                    lump.data.as_slice().read_exact(&mut header).unwrap();
                    //              MIDI                  MUS
                    if &header == b"MThd" || &header == b"MUS\x1A" {
                        wad.music.push(lump);
                        continue;
                    }

                    // TODO check for sound header (doom format)
                }
                // 0-length lumps can be map names or markers
                possible_map_name = lump.name;
            }
        }

        Ok(wad)
    }
}
