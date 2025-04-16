use std::{fs::{self, File}, io::{BufReader, Read}, path::{self, Path, PathBuf}};

use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;
use rosu_map::{section::general::GameMode, Beatmap};
use rusqlite::{params, Connection};

pub const DEFAULT_DB_PATH: &str = "./rosu.db";

#[derive(Debug, Clone)]
pub struct BeatmapEntry {
    pub id: u64,
    pub beatmap_id: i64,
    pub beatmapset_id: i64,
    pub title: String,
    pub artist: String,
    pub creator: String,
    pub version: String,
    pub path: PathBuf,
    pub hash: String,
}

impl TryFrom<&rusqlite::Row<'_>> for BeatmapEntry {
    type Error = rusqlite::Error;

    fn try_from(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let path: String = row.get(7)?;
        Ok(Self {
            id: row.get(0)?,
            beatmap_id: row.get(1)?,
            beatmapset_id: row.get(2)?,
            title: row.get(3)?,
            artist: row.get(4)?,
            creator: row.get(5)?,
            version: row.get(6)?,
            path: PathBuf::from(path),
            hash: row.get(8)?,
        })
    }
}

pub struct OsuDatabase {
    conn: Pool<SqliteConnectionManager>,

    // A in-memory cache for faster loading times
    pub cache: Vec<BeatmapEntry>,
}

impl OsuDatabase {
    // Initial creation of database
    pub fn create_empty_from_path<T: AsRef<Path>>(path: T) -> Result<Pool<SqliteConnectionManager>, rusqlite::Error> {
        let manager = SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::new(manager).unwrap();

        const QUERY: &str = "
            CREATE TABLE beatmaps (
                id INTEGER PRIMARY KEY, 
                beatmapset_id INTEGER, 
                beatmap_id INTEGER, 
                title TEXT, 
                artist TEXT, 
                creator TEXT, 
                version TEXT,
                path TEXT,
                hash TEXT NOT NULL
            );

            CREATE INDEX hash_beatmap
            ON beatmaps(hash);
        ";

        let conn = pool.get().unwrap();

        conn.execute(QUERY, [])?;

        Ok(pool)
    }

    pub fn new_from_path<T: AsRef<Path>>(path: T) -> Result<Self, rusqlite::Error> {
        let pool = if path.as_ref().exists() {
            let manager = SqliteConnectionManager::file(&path);
            let pool = r2d2::Pool::new(manager).unwrap();

            pool
        } else {
            Self::create_empty_from_path(&path)?
        };

        // Setting WAL mode
        {
            let conn = pool.get().unwrap();
            conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        }

        tracing::info!("Initialized DB connection at {:?}", path.as_ref());

        let db = Self {
            cache: Vec::new(),
            conn: pool,
        };

        Ok(db)
    }
    
    // Spawns a job to recursively look for beatmaps in directory
    pub fn scan_beatmaps(&self, look_path: impl AsRef<Path>, stop_rx: oneshot::Receiver<()>) {
        let pool = self.conn.clone();
        let path: PathBuf = look_path.as_ref().to_path_buf();
    
        // TODO: Maybe keep a worker thread around instead of spawning a new one everytime :D
        std::thread::spawn(move || {
            'main_loop: for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();

                if !entry.path().is_dir() {
                    continue;
                }

                for entry in fs::read_dir(entry.path()).unwrap() {
                    let entry = entry.unwrap();

                    if stop_rx.try_recv().is_ok() {
                        break 'main_loop;
                    }

                    // TODO insert in batches
                    if let Some(ext) = entry.path().extension() {
                        if ext == "osu" {
                            let file = File::open(&entry.path()).unwrap();
                            let mut reader = BufReader::new(file);

                            let mut buff = Vec::new();
                            reader.read_to_end(&mut buff).unwrap();

                            let md5_hash = format!("{:x}", md5::compute(&buff));

                            let conn = pool.get().unwrap();
                            if Self::get_beatmap_by_hash_external(&conn, &md5_hash).is_some() {
                                continue;
                            }

                            let beatmap = Beatmap::from_bytes(&buff).unwrap();

                            if beatmap.mode != GameMode::Osu {
                                continue
                            }

                            // raw entry
                            let entry = BeatmapEntry {
                                id: 0,
                                beatmap_id: beatmap.beatmap_id as i64,
                                beatmapset_id: beatmap.beatmap_set_id as i64,
                                title: beatmap.title,
                                artist: beatmap.artist,
                                creator: beatmap.creator,
                                version: beatmap.version,
                                path: entry.path(),
                                hash: md5_hash,
                            };

                            Self::insert_beatmap_external(&conn, &entry);
                        }
                    }
                }

                tracing::info!("Parser .osu: {}", entry.path().display());
            }
        });
    }

    pub fn insert_beatmap_external(
        conn: &Connection, 
        entry: &BeatmapEntry,
    ) {
        const QUERY: &str = "
            INSERT INTO beatmaps 
            (beatmapset_id, beatmap_id, title, artist, creator, version, path, hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ";

        conn.execute(
            QUERY, 
            (
                &entry.beatmapset_id,
                &entry.beatmap_id,
                entry.title.as_str(),
                &entry.artist,
                &entry.creator,
                &entry.version,
                format!("{}", &path::absolute(&entry.path).unwrap().display()),
                &entry.hash
            )
        ).unwrap();
    }

    pub fn beatmaps_amount(&self) -> usize {
        const QUERY: &str = "SELECT COUNT(*) FROM beatmaps";

        let amount = self.conn.get().unwrap().query_row(QUERY, [], |row| {
            Ok(row.get(0).unwrap())
        }).unwrap();

        amount
    }

    pub fn get_beatmap_by_index(&mut self, index: usize) -> Option<BeatmapEntry> {
        const QUERY: &str = "SELECT * FROM beatmaps ORDER BY id ASC LIMIT ?1 OFFSET ?1";

        let entry = self.conn.get().unwrap().query_row(QUERY, [index], |row| {
            BeatmapEntry::try_from(row)
        });

        match entry {
            Ok(entry) => Some(entry),
            Err(e) => match e {
                rusqlite::Error::QueryReturnedNoRows => None,
                _ => {
                    tracing::error!("selecting beatmap by index error");
                    None
                },
            },
        }
    }

    pub fn get_beatmap_by_hash(&self, hash: &str) -> Option<BeatmapEntry> {
        const QUERY: &str = "SELECT * FROM beatmaps WHERE hash = ?1";

        let entry = self.conn.get().unwrap().query_row(QUERY, [hash], |row| {
            BeatmapEntry::try_from(row)
        });

        match entry {
            Ok(entry) => Some(entry),
            Err(e) => match e {
                rusqlite::Error::QueryReturnedNoRows => None,
                _ => {
                    tracing::error!("selecting beatmap by index error");
                    None
                },
            },
        }
    }

    pub fn get_beatmap_by_hash_external(
        conn: &Connection, 
        hash: &str
    ) -> Option<BeatmapEntry> {
        const QUERY: &str = "SELECT * FROM beatmaps WHERE hash = ?1";

        let entry = conn.query_row(QUERY, [hash], |row| {
            BeatmapEntry::try_from(row)
        });

        match entry {
            Ok(entry) => Some(entry),
            Err(e) => match e {
                rusqlite::Error::QueryReturnedNoRows => None,
                _ => {
                    tracing::error!("selecting beatmap by index error");
                    None
                },
            },
        }
    }

    pub fn load_beatmaps_range(&mut self, min: usize, max: usize) {
        const QUERY: &str = 
            "select * from beatmaps order by id ASC LIMIT ?1 OFFSET ?2";

        let conn = self.conn.get().unwrap();

        let mut stmt = conn.prepare(QUERY).unwrap();

        let rows = stmt.query_map(params![max - min, min], |row| {
            BeatmapEntry::try_from(row)
        }).unwrap();

        self.cache.clear();
        for row in rows {
            self.cache.push(row.unwrap());
        }


    }
}
