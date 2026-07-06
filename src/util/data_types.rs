use serenity::all::prelude::TypeMapKey;

pub struct VersionsList;

impl TypeMapKey for VersionsList {
    type Value = Vec<String>;
}

pub struct ServNames;

impl TypeMapKey for ServNames {
    type Value = Vec<String>;
}
