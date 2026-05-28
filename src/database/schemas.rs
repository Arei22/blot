diesel::table! {
    servers (name) {
        id -> BigSerial,
        name -> Text,
        version -> Text,
        crack -> Bool,
        port -> BigInt,
        started -> Bool
    }
}
