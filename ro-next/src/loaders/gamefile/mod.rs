//! Manages archives where game assets are stored and provides convenient
//! methods to retrieve each of them individually. The archives implement the
//! [`Archive`] trait.

mod cache;
mod list;

use core::panic;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use blake3::Hash;
#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, Timer, print_debug};
use ro_next_loaders::{FileLoader, FileNotFoundError};

pub use self::cache::{sync_cache_archive, texture_file_dds_name, video_file_ivf_name};
use self::list::GameArchiveList;
use super::archive::folder::FolderArchive;
use super::archive::native::{NativeArchive, NativeArchiveBuilder};
use super::archive::{Archive, ArchiveType, Compression, Writable, os_specific_path};
use crate::loaders::archive::seven_zip::{SevenZipArchive, SevenZipArchiveBuilder};

pub(crate) const CACHE_FILE_NAME: &str = "cache.7z";
pub(crate) const LUA_ARCHIVE_FILE_NAME: &str = "lua_files.7z";

pub(crate) const TEMPORARY_CACHE_FILE_NAME: &str = "cache.7z.tmp";
pub(crate) const HASH_FILE_PATH: &str = "game_file_hash.txt";

/// Folders that contain loose background music files, relative to the game
/// installation directory.
///
/// The original client keeps its music in a `BGM` folder next to the game
/// archives instead of storing it inside them. Other servers put their music
/// into a `data/mp3` folder. Both are supported.
const LOOSE_MUSIC_FOLDERS: &[&str] = &["BGM", "bgm", "data/mp3", "data/BGM", "data/bgm"];

/// This string is used to derive an initialization vector for the game file
/// hash calculation. We can use this to trigger a de-sync of the cache files of
/// users.
const GAME_FILE_DERIVE_KEY: &str = "ro-next 2025-03-09 14:17:23 game file key v1";

struct LoaderArchive {
    archive: Box<dyn Archive>,
    is_game_archive: bool,
}

/// Type implementing the game file loader.
///
/// Currently, there are two types implementing
/// [`Archive`]:
/// - [`NativeArchive`] - Retrieve assets from GRF files.
/// - [`FolderArchive`] - Retrieve assets from an OS folder.
/// - [`SevenZipArchive`] - Retrieve assets from ZIP files.
#[derive(Default)]
pub struct GameFileLoader {
    archives: RwLock<Vec<LoaderArchive>>,
}

impl FileLoader for GameFileLoader {
    fn get(&self, path: &str) -> Result<Vec<u8>, FileNotFoundError> {
        let lowercase_path = path.to_lowercase();
        self.archives
            .read()
            .unwrap()
            .iter()
            .find_map(|archive| archive.archive.get_file_by_path(&lowercase_path))
            .ok_or_else(|| FileNotFoundError::new(path.to_owned()))
    }
}

impl GameFileLoader {
    /// Tests if a file exists in any of the loaded archives.
    ///
    /// Like [`FileLoader::get`] the path is normalized to lower case: the game
    /// archives use lower case file names (the official Windows client treats
    /// file names case-insensitively), so a lookup for e.g.
    /// `data\texture\verus\ver_h_17.BMP` has to find
    /// `data\texture\verus\ver_h_17.bmp`.
    pub fn file_exists(&self, path: &str) -> bool {
        let lowercase_path = path.to_lowercase();
        self.archives
            .read()
            .unwrap()
            .iter()
            .any(|archive| archive.archive.file_exists(&lowercase_path))
    }

    /// Returns the content of a file from **every** loaded archive that
    /// contains it, in archive priority order (highest priority first).
    ///
    /// This is used by lookups that have to combine data from a lower-priority
    /// archive, e.g. an English translation table that only exists in the
    /// server archive while the base game archive contains the original
    /// (untranslated) table.
    pub fn get_all(&self, path: &str) -> Vec<Vec<u8>> {
        let lowercase_path = path.to_lowercase();

        self.archives
            .read()
            .unwrap()
            .iter()
            .filter_map(|archive| archive.archive.get_file_by_path(&lowercase_path))
            .collect()
    }

    fn add_archive(&self, archive: Box<dyn Archive>, is_game_archive: bool) {
        self.archives.write().unwrap().insert(0, LoaderArchive { archive, is_game_archive });
    }

    fn get_archive_type_by_path(path: &Path) -> ArchiveType {
        if path.is_dir() || path.display().to_string().ends_with('/') {
            ArchiveType::Folder
        } else if let Some(extension) = path.extension()
            && let Some("grf") = extension.to_str()
        {
            ArchiveType::Native
        } else if let Some(extension) = path.extension()
            && let Some("7z") = extension.to_str()
        {
            ArchiveType::SevenZip
        } else {
            panic!("Provided archive must be a directory or have a .grf extension")
        }
    }

    fn load_archive_from_path(path: &str) -> Box<dyn Archive> {
        let path = Path::new(path);

        match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchive::from_path(path)),
            ArchiveType::SevenZip => Box::new(SevenZipArchive::from_path(path)),
        }
    }

    pub fn load_archives_from_settings(&self) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("load game archives");

        let game_archive_list = GameArchiveList::load();

        game_archive_list.archives.iter().for_each(|path| {
            let game_archive = Self::load_archive_from_path(path);
            self.add_archive(game_archive, true);
        });

        self.load_loose_music_folders(&game_archive_list.archives);

        #[cfg(feature = "debug")]
        timer.stop();
    }

    /// Mounts the loose background music folders of the game installation as
    /// additional archives.
    ///
    /// The original client keeps its background music in a `BGM` folder next
    /// to the game archives instead of storing it inside of them. RO-Next is
    /// usually started from a different working directory, so we look next to
    /// every configured game archive (following symlinks) and in the working
    /// directory itself. Every music folder that we find is mounted, which
    /// makes the music available through the normal file loader without
    /// copying the (large) files into the working directory.
    fn load_loose_music_folders(&self, archive_paths: &[String]) {
        let mut base_directories: Vec<PathBuf> = Vec::new();

        for archive_path in archive_paths {
            let Ok(canonical_path) = std::fs::canonicalize(archive_path) else {
                continue;
            };

            let directory = match canonical_path.is_dir() {
                true => canonical_path,
                false => match canonical_path.parent() {
                    Some(parent) => parent.to_path_buf(),
                    None => continue,
                },
            };

            base_directories.push(directory);
        }

        // Also support game installations that keep their music in the working
        // directory.
        base_directories.push(PathBuf::from("."));

        let mut mounted_directories: HashSet<PathBuf> = HashSet::new();

        for base_directory in base_directories {
            for music_folder in LOOSE_MUSIC_FOLDERS {
                let music_directory = base_directory.join(os_specific_path(music_folder));

                let Ok(canonical_music_directory) = std::fs::canonicalize(&music_directory) else {
                    continue;
                };

                if !canonical_music_directory.is_dir() || !mounted_directories.insert(canonical_music_directory.clone()) {
                    continue;
                }

                #[cfg(feature = "debug")]
                print_debug!(
                    "mounting loose music folder {}",
                    canonical_music_directory.display().to_string().magenta()
                );

                let music_archive = Box::new(FolderArchive::from_path(&canonical_music_directory));
                self.add_archive(music_archive, false);
            }
        }
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new_derive_key(GAME_FILE_DERIVE_KEY);
        self.archives
            .read()
            .unwrap()
            .iter()
            .filter(|archive| archive.is_game_archive)
            .for_each(|archive| archive.archive.hash(&mut hasher));
        hasher.finalize()
    }

    pub fn remove_patched_lua_files(&self) {
        if Path::new(LUA_ARCHIVE_FILE_NAME).exists()
            && let Err(error) = std::fs::remove_file(LUA_ARCHIVE_FILE_NAME)
        {
            crate::logging::log_warn!("failed to remove the patched Lua archive `{LUA_ARCHIVE_FILE_NAME}`: {error:?}");
        }
    }

    pub fn load_patched_lua_files(&self) {
        if !Path::new(LUA_ARCHIVE_FILE_NAME).exists() {
            self.patch_lua_files();
        }

        let lua_archive = Self::load_archive_from_path(LUA_ARCHIVE_FILE_NAME);
        self.add_archive(lua_archive, false);
    }

    pub fn get_files_with_extension(&self, extensions: &[&str]) -> Vec<String> {
        let mut files = Vec::new();
        self.archives
            .read()
            .unwrap()
            .iter()
            .for_each(|archive| archive.archive.get_files_with_extension(&mut files, extensions));

        files.sort();
        files.dedup();

        files
    }

    fn patch_lua_files(&self) {
        use lunify::{Format, Settings, unify};

        const LUA_BYTECODE_EXTENSION: &str = ".lub";
        let lua_files = self.get_files_with_extension(&[LUA_BYTECODE_EXTENSION]);

        let path = Path::new(LUA_ARCHIVE_FILE_NAME);
        let mut lua_archive: Box<dyn Writable> = match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchiveBuilder::from_path(path)),
            ArchiveType::SevenZip => Box::new(SevenZipArchiveBuilder::from_path(path)),
        };

        let bytecode_format = Format::default();
        let settings = Settings::default();

        #[cfg(feature = "debug")]
        let mut total_count = lua_files.len();
        #[cfg(feature = "debug")]
        let mut failed_count = 0;

        for file_name in lua_files {
            let bytes = match self.get(&file_name) {
                Ok(bytes) => bytes,
                Err(error) => {
                    crate::logging::warn_once(
                        module_path!(),
                        &file_name,
                        format_args!("failed to extract `{file_name}` from the game archives while patching Lua files: {error:?}"),
                    );

                    #[cfg(feature = "debug")]
                    {
                        failed_count += 1;
                    }

                    continue;
                }
            };

            // Try to unify all bytecode to Lua 5.1 and possibly 64 bit.
            match unify(&bytes, &bytecode_format, &settings) {
                Ok(bytes) => lua_archive.add_file(&file_name, bytes, Compression::Default),
                // If the operation fails the file with this error, the Lua file is not actually a
                // pre-compiled binary but rather a source file, so we can safely ignore it.
                Err(lunify::LunifyError::IncorrectSignature) => {
                    #[cfg(feature = "debug")]
                    {
                        total_count -= 1;
                    }
                }
                Err(error) => {
                    crate::logging::warn_once(
                        module_path!(),
                        &file_name,
                        format_args!("could not upcast the Lua bytecode file `{file_name}`: {error:?}"),
                    );

                    #[cfg(feature = "debug")]
                    {
                        failed_count += 1;
                    }
                }
            }
        }

        #[cfg(feature = "debug")]
        print_debug!(
            "converted a total of {} files of which {} failed.",
            total_count.yellow(),
            failed_count.red(),
        );

        if let Err(error) = lua_archive.finish() {
            crate::logging::log_warn!("can't save the patched Lua archive `{LUA_ARCHIVE_FILE_NAME}`: {error:?}");
        }
    }

    #[allow(unused_variables)]
    pub fn load_cache_archive(&self, game_file_hash: Hash) {
        let path = Path::new(CACHE_FILE_NAME);

        if !path.exists() && !path.is_dir() {
            return;
        }

        let archive = Box::new(SevenZipArchive::from_path(path));

        let Some(hash_file) = archive.get_file_by_path(HASH_FILE_PATH) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't find game hash file. Using empty cache");
            return;
        };

        let Ok(_hash) = Hash::from_hex(hash_file) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't read game hash file. Using empty cache");
            return;
        };

        #[cfg(feature = "debug")]
        if _hash != game_file_hash {
            print_debug!("[{}] Cache is out of sync. Please re-sync or delete the cache", "error".red());
        }

        self.add_archive(archive, false);
    }
}

pub fn fix_broken_texture_file_endings(path: &str) -> String {
    // File names are case-insensitive (the official client runs on Windows), the
    // archives only contain lower case names. Normalizing here also makes the
    // texture cache key deterministic.
    let mut path = path.to_lowercase();

    if path.ends_with(".bm") {
        path.push('p');
    }

    if path.ends_with(".jp") {
        path.push('g');
    }

    if path.ends_with(".pn") {
        path.push('g');
    }

    if path.ends_with(".tg") {
        path.push('a');
    }

    path
}
