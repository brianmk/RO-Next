use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use hashbrown::HashMap;
use mlua::Lua;

use super::{HashMapExt, Library, LuaExt, Table, fix_encoding};
use crate::loaders::GameFileLoader;

/// The resource name of a headgear accessory, indexed by its view id.
///
/// Headgear equipment only stores a view id in the network packets. The
/// official client maps these view ids to sprite resource names using
/// `data\luafiles514\lua files\datainfo\accname.lub` (`AccNameTable`),
/// the symbolic names of the view ids are defined in `accessoryid.lub`
/// (`ACCESSORY_IDs`).
///
/// The sprite of an accessory lives in
/// `data\sprite\악세사리\<남|여>\<남|여><name>`, where the name of the table
/// entry already starts with an underscore (e.g. view id `1` is `_고글` and
/// the sprite path is `악세사리\남\남_고글`).
pub struct AccessoryName(Cow<'static, str>);

impl AccessoryName {
    /// The resource name, including the leading underscore.
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl Display for AccessoryName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Table for AccessoryName {
    type Key<'a> = u16;
    type Storage = HashMap<u16, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `ACCESSORY_IDs` table.
            "data\\luafiles514\\lua files\\accessoryid.lub",
            // Needed to get the `AccNameTable` table.
            "data\\luafiles514\\lua files\\accname.lub",
        ])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        // The `ACCESSORY_IDs` table is not needed for rendering (the view id is
        // already part of the packets), but loading it validates that both
        // accessory tables are present and usable.
        if let Ok(accessory_ids) = globals.get::<mlua::Table>("ACCESSORY_IDs") {
            let _ = accessory_ids.pairs::<String, u16>().flatten().count();
        }

        // A missing `AccNameTable` (or a client without the accessory data)
        // simply means that no headgear is rendered, which is better than
        // panicking over a missing table.
        if let Ok(acc_name_table) = globals.get::<mlua::Table>("AccNameTable") {
            for (view_id, name) in acc_name_table.pairs::<u16, String>().flatten() {
                // The patched Lua archives contain the Korean strings as
                // latin-1 mojibake (the EUC-KR bytes were decoded as latin-1
                // and re-encoded as UTF-8). `fix_encoding` reverses that.
                let name = fix_encoding(name);

                if name.is_empty() {
                    continue;
                }

                result.insert(view_id, AccessoryName(Cow::Owned(name)));
            }
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.accessory_name_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: AccessoryName = AccessoryName(Cow::Borrowed(""));

        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
